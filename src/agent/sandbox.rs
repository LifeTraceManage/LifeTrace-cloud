use std::collections::HashSet;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rig::tool::{MissingToolContext, Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::agent::approvals::{propose, AgentApproval, ApprovalError};
use crate::agent::context::AgentInvocationContext;

const DEFAULT_JOBS_DIR: &str = "/data/agent-jobs";
const DEFAULT_RUNS_DIR: &str = "/data/agent-sandbox-runs";
const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const DEFAULT_MAX_TIMEOUT_SECONDS: u64 = 120;
const DEFAULT_MAX_INPUT_BYTES: usize = 32 * 1024;
const DEFAULT_MAX_OUTPUT_BYTES: usize = 64 * 1024;

static RUNNING_APPROVALS: OnceLock<tokio::sync::Mutex<HashSet<Uuid>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct SandboxSettings {
    enabled: bool,
    jobs_dir: PathBuf,
    runs_dir: PathBuf,
    max_timeout_seconds: u64,
    max_input_bytes: usize,
    max_output_bytes: usize,
}

impl SandboxSettings {
    fn from_env() -> Self {
        Self {
            enabled: env_bool("AGENT_SANDBOX_ENABLED", false),
            jobs_dir: PathBuf::from(
                std::env::var("AGENT_SANDBOX_JOBS_DIR")
                    .unwrap_or_else(|_| DEFAULT_JOBS_DIR.to_owned()),
            ),
            runs_dir: PathBuf::from(
                std::env::var("AGENT_SANDBOX_RUNS_DIR")
                    .unwrap_or_else(|_| DEFAULT_RUNS_DIR.to_owned()),
            ),
            max_timeout_seconds: env_u64(
                "AGENT_SANDBOX_MAX_TIMEOUT_SECONDS",
                DEFAULT_MAX_TIMEOUT_SECONDS,
            )
            .clamp(1, 600),
            max_input_bytes: env_usize(
                "AGENT_SANDBOX_MAX_INPUT_BYTES",
                DEFAULT_MAX_INPUT_BYTES,
            )
            .clamp(1024, 1024 * 1024),
            max_output_bytes: env_usize(
                "AGENT_SANDBOX_MAX_OUTPUT_BYTES",
                DEFAULT_MAX_OUTPUT_BYTES,
            )
            .clamp(1024, 1024 * 1024),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SandboxJobManifest {
    id: String,
    description: String,
    entrypoint: String,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    input_schema: Option<Value>,
}

#[derive(Debug, Clone)]
struct LoadedJob {
    manifest: SandboxJobManifest,
    executable: PathBuf,
    revision: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SandboxJobSummary {
    id: String,
    description: String,
    timeout_seconds: u64,
    input_schema: Value,
    revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSandboxJobArgs {
    pub job_id: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunSandboxJobAction {
    job_id: String,
    input: Value,
    revision: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("missing agent invocation context: {0}")]
    Context(#[from] MissingToolContext),
    #[error("agent sandbox is disabled")]
    Disabled,
    #[error("sandbox job not found: {0}")]
    NotFound(String),
    #[error("invalid sandbox job: {0}")]
    Invalid(String),
    #[error("sandbox I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("sandbox process failed: {0}")]
    Process(String),
    #[error("sandbox job timed out after {0} seconds")]
    Timeout(u64),
}

pub struct ListSandboxJobsTool;
pub struct ProposeRunSandboxJobTool;

impl Tool for ListSandboxJobsTool {
    const NAME: &'static str = "lifetrace_list_sandbox_jobs";
    type Args = ();
    type Output = Value;
    type Error = SandboxError;

    fn description(&self) -> String {
        "列出服务器管理员已注册、允许 Agent 调用的沙盒 Job。这里只能看到白名单 Job；不存在任意 shell/命令执行能力。只读。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        _args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?;
        if !ctx.scopes.contains("execution:read") {
            return Err(SandboxError::Invalid(
                "execution:read scope is required".to_owned(),
            ));
        }

        let settings = SandboxSettings::from_env();
        if !settings.enabled {
            return Ok(json!({
                "enabled": false,
                "items": [],
                "security": {
                    "shell": false,
                    "arbitraryExecutable": false,
                    "environmentCleared": true,
                    "approvalRequired": true
                }
            }));
        }

        let jobs = list_jobs(&settings)?;
        Ok(json!({
            "enabled": true,
            "items": jobs,
            "security": {
                "shell": false,
                "arbitraryExecutable": false,
                "environmentCleared": true,
                "approvalRequired": true
            }
        }))
    }
}

impl Tool for ProposeRunSandboxJobTool {
    const NAME: &'static str = "lifetrace_propose_run_sandbox_job";
    type Args = RunSandboxJobArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出执行一个管理员预先注册的沙盒 Job。必须先调用 lifetrace_list_sandbox_jobs 获取准确 jobId 和输入约束；不会直接运行，只有用户批准后才执行。不能传 shell 命令、可执行路径或脚本内容。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "jobId":{"type":"string","description":"必须来自 lifetrace_list_sandbox_jobs"},
                "input":{"type":"object","description":"传给 Job 的 JSON 输入；不会拼接到 shell 命令"},
                "supersedesApprovalId":{"type":"string","description":"可选。修改尚未批准的同一 Job 提案时填写旧 approvalId"}
            },
            "required":["jobId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        if !ctx.scopes.contains("execution:write") {
            return Err(ApprovalError::Permission("execution:write".to_owned()));
        }

        let settings = SandboxSettings::from_env();
        if !settings.enabled {
            return Err(ApprovalError::Invalid(
                "agent sandbox is disabled by server configuration".to_owned(),
            ));
        }

        let job_id = validate_job_id(&args.job_id)
            .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
        let job = load_job(&settings, &job_id)
            .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
        validate_input_size(&settings, &args.input)
            .map_err(|error| ApprovalError::Invalid(error.to_string()))?;

        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let action = json!({
            "jobId": job.manifest.id,
            "input": args.input,
            "revision": job.revision
        });
        let preview = json!({
            "jobId": job.manifest.id,
            "description": job.manifest.description,
            "input": action.get("input").cloned().unwrap_or_else(|| json!({})),
            "revision": job.revision
        });

        propose(
            &ctx,
            Self::NAME,
            "run_sandbox_job",
            arguments_json,
            action,
            preview,
            args.supersedes_approval_id.as_deref(),
        )
        .await
    }
}

pub async fn execute_approved_job(approval: &AgentApproval) -> Result<Value, SandboxError> {
    let action: RunSandboxJobAction = serde_json::from_value(approval.action_json.clone())
        .map_err(|error| SandboxError::Invalid(error.to_string()))?;
    let settings = SandboxSettings::from_env();
    if !settings.enabled {
        return Err(SandboxError::Disabled);
    }

    let running = RUNNING_APPROVALS.get_or_init(|| tokio::sync::Mutex::new(HashSet::new()));
    {
        let mut guard = running.lock().await;
        if !guard.insert(approval.id) {
            return Err(SandboxError::Process(
                "this approval is already executing".to_owned(),
            ));
        }
    }

    let result = execute_job_inner(&settings, approval.id, action).await;
    running.lock().await.remove(&approval.id);
    result
}

async fn execute_job_inner(
    settings: &SandboxSettings,
    execution_id: Uuid,
    action: RunSandboxJobAction,
) -> Result<Value, SandboxError> {
    let job_id = validate_job_id(&action.job_id)?;
    let job = load_job(settings, &job_id)?;
    if job.revision != action.revision {
        return Err(SandboxError::Invalid(
            "sandbox job changed after approval was proposed; create a new proposal".to_owned(),
        ));
    }
    validate_input_size(settings, &action.input)?;

    tokio::fs::create_dir_all(&settings.runs_dir).await?;
    let run_dir = settings.runs_dir.join(execution_id.to_string());
    match tokio::fs::create_dir(&run_dir).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(SandboxError::Process(
                "sandbox execution directory already exists".to_owned(),
            ));
        }
        Err(error) => return Err(error.into()),
    }

    let input_path = run_dir.join("input.json");
    let output_path = run_dir.join("output.json");
    let encoded_input = serde_json::to_vec(&action.input)
        .map_err(|error| SandboxError::Invalid(error.to_string()))?;
    tokio::fs::write(&input_path, encoded_input).await?;

    let timeout_seconds = job
        .manifest
        .timeout_seconds
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
        .clamp(1, settings.max_timeout_seconds);
    let started = Instant::now();

    let mut command = Command::new(&job.executable);
    command
        .current_dir(&run_dir)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("TZ", "UTC")
        .env("HOME", &run_dir)
        .env("TMPDIR", &run_dir)
        .env("LIFETRACE_SANDBOX_JOB_ID", &job.manifest.id)
        .env("LIFETRACE_SANDBOX_EXECUTION_ID", execution_id.to_string())
        .env("LIFETRACE_SANDBOX_INPUT", &input_path)
        .env("LIFETRACE_SANDBOX_OUTPUT", &output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|error| SandboxError::Process(format!("failed to start job: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SandboxError::Process("stdout pipe is unavailable".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SandboxError::Process("stderr pipe is unavailable".to_owned()))?;

    let max_output = settings.max_output_bytes;
    let stdout_task = tokio::spawn(read_limited(stdout, max_output));
    let stderr_task = tokio::spawn(read_limited(stderr, max_output));

    let status = match tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        child.wait(),
    )
    .await
    {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            let _ = child.kill().await;
            cleanup_run_dir(&run_dir).await;
            return Err(SandboxError::Process(format!(
                "failed while waiting for job: {error}"
            )));
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            cleanup_run_dir(&run_dir).await;
            return Err(SandboxError::Timeout(timeout_seconds));
        }
    };

    let (stdout, stdout_truncated) = join_capture(stdout_task, "stdout").await?;
    let (stderr, stderr_truncated) = join_capture(stderr_task, "stderr").await?;
    let structured_output = read_structured_output(&output_path, settings.max_output_bytes).await?;
    let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    cleanup_run_dir(&run_dir).await;

    let exit_code = status.code();
    if !status.success() {
        return Err(SandboxError::Process(format!(
            "job exited unsuccessfully (code {:?}): {}",
            exit_code,
            if stderr.trim().is_empty() {
                stdout.chars().take(500).collect::<String>()
            } else {
                stderr.chars().take(500).collect::<String>()
            }
        )));
    }

    Ok(json!({
        "action": "run_sandbox_job",
        "jobId": job.manifest.id,
        "revision": job.revision,
        "executionId": execution_id,
        "exitCode": exit_code,
        "durationMs": duration_ms,
        "output": structured_output,
        "stdout": stdout,
        "stderr": stderr,
        "stdoutTruncated": stdout_truncated,
        "stderrTruncated": stderr_truncated
    }))
}

async fn read_limited<R>(reader: R, limit: usize) -> Result<(Vec<u8>, bool), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = reader.take((limit.saturating_add(1)) as u64);
    let mut bytes = Vec::with_capacity(limit.min(8192));
    reader.read_to_end(&mut bytes).await?;
    let truncated = bytes.len() > limit;
    if truncated {
        bytes.truncate(limit);
    }
    Ok((bytes, truncated))
}

async fn join_capture(
    task: tokio::task::JoinHandle<Result<(Vec<u8>, bool), std::io::Error>>,
    stream: &str,
) -> Result<(String, bool), SandboxError> {
    let (bytes, truncated) = task
        .await
        .map_err(|error| SandboxError::Process(format!("{stream} reader failed: {error}")))??;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

async fn read_structured_output(path: &Path, max_bytes: usize) -> Result<Option<Value>, SandboxError> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_file() {
        return Err(SandboxError::Invalid(
            "sandbox output must be a regular file".to_owned(),
        ));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(SandboxError::Invalid(format!(
            "sandbox output.json exceeds {max_bytes} bytes"
        )));
    }
    let bytes = tokio::fs::read(path).await?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| SandboxError::Invalid(format!("output.json is not valid JSON: {error}")))?;
    Ok(Some(value))
}

async fn cleanup_run_dir(path: &Path) {
    if let Err(error) = tokio::fs::remove_dir_all(path).await {
        tracing::warn!(path = %path.display(), error = %error, "failed to clean sandbox run directory");
    }
}

fn list_jobs(settings: &SandboxSettings) -> Result<Vec<SandboxJobSummary>, SandboxError> {
    let root = canonical_jobs_root(settings)?;
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if validate_job_id(stem).is_ok() {
            ids.push(stem.to_owned());
        }
    }
    ids.sort();

    let mut jobs = Vec::with_capacity(ids.len());
    for id in ids {
        match load_job(settings, &id) {
            Ok(job) => jobs.push(SandboxJobSummary {
                id: job.manifest.id,
                description: job.manifest.description,
                timeout_seconds: job
                    .manifest
                    .timeout_seconds
                    .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
                    .clamp(1, settings.max_timeout_seconds),
                input_schema: job
                    .manifest
                    .input_schema
                    .unwrap_or_else(|| json!({"type":"object"})),
                revision: job.revision,
            }),
            Err(error) => {
                tracing::warn!(job_id = %id, error = %error, "ignoring invalid sandbox job manifest");
            }
        }
    }
    Ok(jobs)
}

fn load_job(settings: &SandboxSettings, job_id: &str) -> Result<LoadedJob, SandboxError> {
    let job_id = validate_job_id(job_id)?;
    let root = canonical_jobs_root(settings)?;
    let manifest_path = root.join(format!("{job_id}.json"));
    let canonical_manifest = std::fs::canonicalize(&manifest_path)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SandboxError::NotFound(job_id.clone()),
            _ => SandboxError::Io(error),
        })?;
    ensure_inside(&root, &canonical_manifest, "manifest")?;

    let manifest_bytes = std::fs::read(&canonical_manifest)?;
    if manifest_bytes.len() > 64 * 1024 {
        return Err(SandboxError::Invalid(
            "job manifest exceeds 64 KiB".to_owned(),
        ));
    }
    let manifest: SandboxJobManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| SandboxError::Invalid(format!("invalid manifest JSON: {error}")))?;
    if manifest.id != job_id {
        return Err(SandboxError::Invalid(
            "manifest id must match its filename".to_owned(),
        ));
    }
    if manifest.description.trim().is_empty() || manifest.description.chars().count() > 500 {
        return Err(SandboxError::Invalid(
            "description must contain 1-500 characters".to_owned(),
        ));
    }

    let relative_entrypoint = PathBuf::from(&manifest.entrypoint);
    if relative_entrypoint.as_os_str().is_empty()
        || relative_entrypoint.is_absolute()
        || relative_entrypoint.components().any(|component| {
            !matches!(component, Component::Normal(_))
        })
    {
        return Err(SandboxError::Invalid(
            "entrypoint must be a relative path without traversal".to_owned(),
        ));
    }

    let executable = std::fs::canonicalize(root.join(relative_entrypoint))
        .map_err(|error| SandboxError::Invalid(format!("entrypoint is unavailable: {error}")))?;
    ensure_inside(&root, &executable, "entrypoint")?;
    let metadata = std::fs::metadata(&executable)?;
    if !metadata.is_file() {
        return Err(SandboxError::Invalid(
            "entrypoint must be a regular file".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(SandboxError::Invalid(
                "entrypoint must have an executable bit".to_owned(),
            ));
        }
    }

    let revision = calculate_revision(&manifest_bytes, &executable)?;
    Ok(LoadedJob {
        manifest,
        executable,
        revision,
    })
}

fn calculate_revision(manifest: &[u8], executable: &Path) -> Result<String, SandboxError> {
    let mut hasher = Sha256::new();
    hasher.update(manifest);
    hasher.update([0]);
    let file = std::fs::File::open(executable)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn canonical_jobs_root(settings: &SandboxSettings) -> Result<PathBuf, SandboxError> {
    let root = std::fs::canonicalize(&settings.jobs_dir).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SandboxError::Invalid(format!(
            "sandbox jobs directory does not exist: {}",
            settings.jobs_dir.display()
        )),
        _ => SandboxError::Io(error),
    })?;
    if !root.is_dir() {
        return Err(SandboxError::Invalid(
            "sandbox jobs path must be a directory".to_owned(),
        ));
    }
    Ok(root)
}

fn ensure_inside(root: &Path, path: &Path, label: &str) -> Result<(), SandboxError> {
    if !path.starts_with(root) {
        return Err(SandboxError::Invalid(format!(
            "{label} resolves outside the sandbox jobs directory"
        )));
    }
    Ok(())
}

fn validate_job_id(value: &str) -> Result<String, SandboxError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 64 {
        return Err(SandboxError::Invalid(
            "jobId must contain 1-64 characters".to_owned(),
        ));
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(SandboxError::Invalid("jobId is empty".to_owned()));
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(SandboxError::Invalid(
            "jobId must start with a lowercase letter or digit".to_owned(),
        ));
    }
    if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        return Err(SandboxError::Invalid(
            "jobId may contain only lowercase letters, digits, '-' and '_'".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_input_size(settings: &SandboxSettings, input: &Value) -> Result<(), SandboxError> {
    if !input.is_object() {
        return Err(SandboxError::Invalid(
            "sandbox job input must be a JSON object".to_owned(),
        ));
    }
    let size = serde_json::to_vec(input)
        .map_err(|error| SandboxError::Invalid(error.to_string()))?
        .len();
    if size > settings.max_input_bytes {
        return Err(SandboxError::Invalid(format!(
            "sandbox input exceeds {} bytes",
            settings.max_input_bytes
        )));
    }
    Ok(())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_job_ids() {
        assert_eq!(validate_job_id("backup_daily").unwrap(), "backup_daily");
        assert_eq!(validate_job_id("job-2").unwrap(), "job-2");
        assert!(validate_job_id("../backup").is_err());
        assert!(validate_job_id("Backup").is_err());
        assert!(validate_job_id("job name").is_err());
    }

    #[test]
    fn rejects_non_object_input() {
        let settings = SandboxSettings {
            enabled: true,
            jobs_dir: PathBuf::from("/tmp/jobs"),
            runs_dir: PathBuf::from("/tmp/runs"),
            max_timeout_seconds: 60,
            max_input_bytes: 128,
            max_output_bytes: 1024,
        };
        assert!(validate_input_size(&settings, &json!({"ok": true})).is_ok());
        assert!(validate_input_size(&settings, &json!(["not", "object"])).is_err());
    }

    #[test]
    fn rejects_traversal_entrypoint_components() {
        let path = PathBuf::from("bin/../secret");
        assert!(path
            .components()
            .any(|component| !matches!(component, Component::Normal(_))));
    }
}
