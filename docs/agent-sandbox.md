# LifeTrace Agent Sandbox Jobs

LifeTrace Agent 的沙盒能力是一个**受控 Job Runner**，不是通用 Shell。

Agent 只能：

1. 读取服务器管理员预先注册的 Job；
2. 根据 Job 的输入说明生成 JSON 参数；
3. 创建 `run_sandbox_job` 审批；
4. 用户批准后，由 Cloud 执行固定的 entrypoint；
5. 将 `output.json`、stdout 和 stderr 返回并写入当前 Agent 会话。

Agent 不能提交可执行路径、Shell 命令、脚本正文，也不能通过这个能力直接调用 `docker`、`git` 或任意系统命令。

## 启用

生产环境默认关闭：

```env
AGENT_SANDBOX_ENABLED=false
AGENT_SANDBOX_JOBS_DIR=/data/agent-jobs
AGENT_SANDBOX_RUNS_DIR=/data/agent-sandbox-runs
AGENT_SANDBOX_MAX_TIMEOUT_SECONDS=120
AGENT_SANDBOX_MAX_INPUT_BYTES=32768
AGENT_SANDBOX_MAX_OUTPUT_BYTES=65536
```

准备好 Job 后再设置：

```env
AGENT_SANDBOX_ENABLED=true
```

然后重新部署 Cloud。启用后，Cloud 会自动确保 `AGENT_SANDBOX_JOBS_DIR` 存在，因此已有的 `/data` volume 不需要手工预建目录；Job manifest 和 entrypoint 仍需由服务器管理员部署。

## 注册 Job

每个 Job 由一个 manifest 和一个可执行文件组成。manifest 文件名必须与 `id` 一致。

例如：

```text
/data/agent-jobs/
├── summarize_file.json
└── bin/
    └── summarize_file.sh
```

`/data/agent-jobs/summarize_file.json`：

```json
{
  "id": "summarize_file",
  "description": "读取管理员允许的数据源并生成摘要",
  "entrypoint": "bin/summarize_file.sh",
  "timeoutSeconds": 30,
  "inputSchema": {
    "type": "object",
    "properties": {
      "fileId": {
        "type": "string",
        "description": "LifeTrace 文件 ID"
      }
    },
    "required": ["fileId"],
    "additionalProperties": false
  }
}
```

entrypoint 必须：

- 位于 `AGENT_SANDBOX_JOBS_DIR` 内；
- 使用相对路径；
- 不能包含 `..`；
- 是普通文件；
- 在 Linux 上具有 executable bit。

例如：

```bash
chmod +x /data/agent-jobs/bin/summarize_file.sh
```

## Job 输入/输出协议

Cloud **不会**把模型生成的参数拼接到命令行。

执行时不会调用 `/bin/sh -c`，而是直接启动 manifest 中固定的 entrypoint。用户输入会被序列化到 JSON 文件。

Job 通过以下环境变量获取运行信息：

```text
LIFETRACE_SANDBOX_JOB_ID
LIFETRACE_SANDBOX_EXECUTION_ID
LIFETRACE_SANDBOX_INPUT
LIFETRACE_SANDBOX_OUTPUT
```

其中：

- `LIFETRACE_SANDBOX_INPUT`：输入 JSON 文件路径；
- `LIFETRACE_SANDBOX_OUTPUT`：可选结构化结果文件路径；
- stdout / stderr 也会被捕获；
- output 与 stdout/stderr 都有大小上限。

示例脚本：

```sh
#!/bin/sh
set -eu

INPUT="$LIFETRACE_SANDBOX_INPUT"
OUTPUT="$LIFETRACE_SANDBOX_OUTPUT"

# 这里只是协议示例。实际 Job 应自行严格校验 input.json。
printf '{"ok":true,"message":"job completed"}\n' > "$OUTPUT"
printf 'processed input at %s\n' "$INPUT"
```

## 执行安全策略

当前实现包含以下边界：

- 默认关闭；
- Job 白名单；
- Job ID 严格校验；
- entrypoint 必须留在 jobs root；
- 不接受任意可执行路径；
- 不使用 Shell 拼接用户输入；
- 执行前必须走 Agent Approval；
- Proposal 保存 Job revision；
- 批准执行时重新计算 manifest + entrypoint SHA-256，发生变化则拒绝执行；
- 每次运行创建独立临时工作目录；
- 子进程环境变量先 `env_clear`，不会继承数据库密钥、模型 Key、邮件凭证等 Cloud secrets；
- stdin 关闭；
- 执行超时；
- stdout / stderr / output 大小限制；
- 同一个 Approval 防止进程内并发重复运行；
- Tool Call、Approval 与执行结果沿用现有 Agent 审计记录。

## 重要边界

这是一层**应用级受控执行沙盒**，用于防止 LLM 获得任意 Shell 权限。

它不是用于运行不可信恶意代码的 VM/容器安全边界。管理员注册的 entrypoint 与 LifeTrace Cloud 运行在同一个 Cloud 容器用户下，因此 Job 本身仍应被视为受信任代码。

当前 V1 **没有承诺内核级网络隔离或文件系统 namespace 隔离**。

如果未来需要让 Agent 生成并运行不可信代码，应该增加独立执行后端，例如：

- WASI / Wasmtime；
- gVisor；
- Firecracker microVM；
- 独立无网络 OCI sandbox worker。

不要把通用 `shell(command)` Tool 加入当前 Agent Runtime。
