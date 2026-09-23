//! Strongly typed cloud and authentication configuration.

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: String,
    pub bind_addr: SocketAddr,
    pub database_path: String,
    pub migration_on_startup: bool,

    pub request_body_limit_bytes: usize,
    pub push_max_changes: usize,
    pub pull_max_changes: usize,
    pub snapshot_max_page_size: usize,
    pub maximum_atomic_group_size: usize,
    pub cursor_signing_key: Option<String>,
    pub page_token_signing_key: Option<String>,
    pub cors_allowed_origins: Vec<String>,

    /// Development-only fixed Bearer credential retained for in-process tests.
    pub dev_auth_enabled: bool,
    pub dev_auth_user_id: String,
    pub dev_auth_device_id: String,
    pub dev_auth_token: String,

    pub auth_registration_mode: String,
    pub auth_access_token_ttl_seconds: u64,
    pub auth_refresh_idle_ttl_seconds: u64,
    pub auth_refresh_absolute_ttl_seconds: u64,
    pub auth_web_idle_ttl_seconds: u64,
    pub auth_web_absolute_ttl_seconds: u64,
    pub auth_public_device_ttl_seconds: u64,
    pub auth_argon2_memory_kib: u32,
    pub auth_argon2_iterations: u32,
    pub auth_argon2_parallelism: u32,
    pub auth_password_min_length: usize,
    pub auth_password_max_bytes: usize,
    pub auth_password_blocklist_path: Option<String>,
    pub auth_password_pepper: Option<String>,
    pub auth_token_hash_pepper: Option<String>,
    pub auth_reset_token_ttl_seconds: u64,
    pub auth_login_account_limit: usize,
    pub auth_login_ip_limit: usize,
    pub auth_login_window_seconds: u64,
    pub auth_lockout_seconds: u64,
    pub auth_cookie_name: String,
    pub auth_cookie_same_site: String,
    pub auth_cookie_secure: bool,
    pub auth_trusted_proxy_cidrs: Vec<String>,
    pub auth_reset_notifier: String,
    pub public_web_base_url: Option<String>,

    pub snapshot_ttl_seconds: u64,
    pub maintenance_interval_seconds: u64,
    pub graceful_shutdown_seconds: u64,
    pub retention_entries: usize,

    pub beecount_attachment_max_upload_bytes: usize,

    pub file_object_storage_endpoint: Option<String>,
    pub file_object_storage_bucket: Option<String>,
    pub file_object_storage_region: String,
    pub file_object_storage_access_key_id: Option<String>,
    pub file_object_storage_secret_access_key: Option<String>,
    pub file_object_storage_presign_ttl_seconds: u32,
    pub file_max_upload_bytes: i64,

    pub mail_credential_key: Option<String>,

    pub deepseek_api_key: Option<String>,
    pub deepseek_base_url: String,
    pub deepseek_model: String,

    pub zhipu_api_key: Option<String>,
    pub zhipu_base_url: String,
    pub photo_challenge_model: String,
    pub photo_challenge_access_key: Option<String>,
    pub photo_challenge_owner_email: Option<String>,
    pub photo_staging_ttl_hours: Option<i64>,
    pub photo_staging_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            environment: "development".to_owned(),
            bind_addr: "127.0.0.1:8787".parse().expect("static addr"),
            database_path: "./data/lifetrace.db".to_owned(),
            migration_on_startup: true,
            request_body_limit_bytes: 4 * 1024 * 1024,
            push_max_changes: 500,
            pull_max_changes: 200,
            snapshot_max_page_size: 200,
            maximum_atomic_group_size: 50,
            cursor_signing_key: Some("dev-cursor-key".to_owned()),
            page_token_signing_key: Some("dev-page-token-key".to_owned()),
            cors_allowed_origins: Vec::new(),
            dev_auth_enabled: true,
            dev_auth_user_id: "dev-user".to_owned(),
            dev_auth_device_id: "dev-device".to_owned(),
            dev_auth_token: "dev-token".to_owned(),
            auth_registration_mode: "disabled".to_owned(),
            auth_access_token_ttl_seconds: 15 * 60,
            auth_refresh_idle_ttl_seconds: 30 * 24 * 60 * 60,
            auth_refresh_absolute_ttl_seconds: 90 * 24 * 60 * 60,
            auth_web_idle_ttl_seconds: 12 * 60 * 60,
            auth_web_absolute_ttl_seconds: 7 * 24 * 60 * 60,
            auth_public_device_ttl_seconds: 8 * 60 * 60,
            auth_argon2_memory_kib: 19_456,
            auth_argon2_iterations: 2,
            auth_argon2_parallelism: 1,
            auth_password_min_length: 9,
            auth_password_max_bytes: 512,
            auth_password_blocklist_path: None,
            auth_password_pepper: Some("development-password-pepper".to_owned()),
            auth_token_hash_pepper: Some("development-token-pepper".to_owned()),
            auth_reset_token_ttl_seconds: 30 * 60,
            auth_login_account_limit: 5,
            auth_login_ip_limit: 30,
            auth_login_window_seconds: 15 * 60,
            auth_lockout_seconds: 15 * 60,
            auth_cookie_name: "__Host-lifetrace_session".to_owned(),
            auth_cookie_same_site: "Lax".to_owned(),
            auth_cookie_secure: false,
            auth_trusted_proxy_cidrs: Vec::new(),
            auth_reset_notifier: "console".to_owned(),
            public_web_base_url: Some("http://127.0.0.1:8787".to_owned()),
            snapshot_ttl_seconds: 3600,
            maintenance_interval_seconds: 300,
            graceful_shutdown_seconds: 10,
            retention_entries: 1000,
            beecount_attachment_max_upload_bytes: 64 * 1024 * 1024,
            file_object_storage_endpoint: None,
            file_object_storage_bucket: None,
            file_object_storage_region: "us-east-1".to_owned(),
            file_object_storage_access_key_id: None,
            file_object_storage_secret_access_key: None,
            file_object_storage_presign_ttl_seconds: 900,
            file_max_upload_bytes: 256 * 1024 * 1024,
            mail_credential_key: None,
            deepseek_api_key: None,
            deepseek_base_url: "https://api.deepseek.com".to_owned(),
            deepseek_model: "deepseek-chat".to_owned(),
            zhipu_api_key: None,
            zhipu_base_url: "https://open.bigmodel.cn/api/paas/v4".to_owned(),
            photo_challenge_model: "glm-4v-flash".to_owned(),
            photo_challenge_access_key: None,
            photo_challenge_owner_email: None,
            photo_staging_ttl_hours: None,
            photo_staging_dir: "./data/photo-staging".to_owned(),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.environment = env_string("LIFETRACE_ENV", &c.environment).to_ascii_lowercase();
        if let Some(value) = env_var("LIFETRACE_BIND_ADDRESS") {
            if let Ok(addr) = value.parse() {
                c.bind_addr = addr;
            }
        }
        c.database_path = env_var("LIFETRACE_DATABASE_PATH").unwrap_or_else(|| {
            if c.is_production() {
                "/data/lifetrace.db".to_owned()
            } else {
                c.database_path.clone()
            }
        });
        c.migration_on_startup = env_bool("MIGRATION_ON_STARTUP", c.migration_on_startup);
        c.request_body_limit_bytes =
            env_usize("REQUEST_BODY_LIMIT_BYTES", c.request_body_limit_bytes);
        c.push_max_changes = env_usize("PUSH_MAX_CHANGES", c.push_max_changes);
        c.pull_max_changes = env_usize("PULL_MAX_CHANGES", c.pull_max_changes);
        c.snapshot_max_page_size = env_usize("SNAPSHOT_MAX_PAGE_SIZE", c.snapshot_max_page_size);
        c.maximum_atomic_group_size =
            env_usize("MAXIMUM_ATOMIC_GROUP_SIZE", c.maximum_atomic_group_size);
        c.cursor_signing_key = env_var("CURSOR_SIGNING_KEY").or(c.cursor_signing_key);
        c.page_token_signing_key = env_var("PAGE_TOKEN_SIGNING_KEY").or(c.page_token_signing_key);
        c.cors_allowed_origins = env_csv("CORS_ALLOWED_ORIGINS", c.cors_allowed_origins);
        c.dev_auth_enabled = env_bool("DEV_AUTH_ENABLED", c.dev_auth_enabled);
        c.dev_auth_user_id = env_string("DEV_AUTH_USER_ID", &c.dev_auth_user_id);
        c.dev_auth_device_id = env_string("DEV_AUTH_DEVICE_ID", &c.dev_auth_device_id);
        c.dev_auth_token = env_string("DEV_AUTH_TOKEN", &c.dev_auth_token);

        c.auth_registration_mode =
            env_string("AUTH_REGISTRATION_MODE", &c.auth_registration_mode).to_ascii_lowercase();
        c.auth_access_token_ttl_seconds = env_u64(
            "AUTH_ACCESS_TOKEN_TTL_SECONDS",
            c.auth_access_token_ttl_seconds,
        );
        c.auth_refresh_idle_ttl_seconds = env_u64(
            "AUTH_REFRESH_IDLE_TTL_SECONDS",
            c.auth_refresh_idle_ttl_seconds,
        );
        c.auth_refresh_absolute_ttl_seconds = env_u64(
            "AUTH_REFRESH_ABSOLUTE_TTL_SECONDS",
            c.auth_refresh_absolute_ttl_seconds,
        );
        c.auth_web_idle_ttl_seconds =
            env_u64("AUTH_WEB_IDLE_TTL_SECONDS", c.auth_web_idle_ttl_seconds);
        c.auth_web_absolute_ttl_seconds = env_u64(
            "AUTH_WEB_ABSOLUTE_TTL_SECONDS",
            c.auth_web_absolute_ttl_seconds,
        );
        c.auth_public_device_ttl_seconds = env_u64(
            "AUTH_PUBLIC_DEVICE_TTL_SECONDS",
            c.auth_public_device_ttl_seconds,
        );
        c.auth_argon2_memory_kib =
            env_usize("AUTH_ARGON2_MEMORY_KIB", c.auth_argon2_memory_kib as usize) as u32;
        c.auth_argon2_iterations =
            env_usize("AUTH_ARGON2_ITERATIONS", c.auth_argon2_iterations as usize) as u32;
        c.auth_argon2_parallelism = env_usize(
            "AUTH_ARGON2_PARALLELISM",
            c.auth_argon2_parallelism as usize,
        ) as u32;
        c.auth_password_min_length =
            env_usize("AUTH_PASSWORD_MIN_LENGTH", c.auth_password_min_length);
        c.auth_password_max_bytes = env_usize("AUTH_PASSWORD_MAX_BYTES", c.auth_password_max_bytes);
        c.auth_password_blocklist_path = env_var("AUTH_PASSWORD_BLOCKLIST_PATH");
        c.auth_password_pepper = env_var("AUTH_PASSWORD_PEPPER").or(c.auth_password_pepper);
        c.auth_token_hash_pepper = env_var("AUTH_TOKEN_HASH_PEPPER").or(c.auth_token_hash_pepper);
        c.auth_reset_token_ttl_seconds = env_u64(
            "AUTH_RESET_TOKEN_TTL_SECONDS",
            c.auth_reset_token_ttl_seconds,
        );
        c.auth_login_account_limit =
            env_usize("AUTH_LOGIN_ACCOUNT_LIMIT", c.auth_login_account_limit);
        c.auth_login_ip_limit = env_usize("AUTH_LOGIN_IP_LIMIT", c.auth_login_ip_limit);
        c.auth_login_window_seconds =
            env_u64("AUTH_LOGIN_WINDOW_SECONDS", c.auth_login_window_seconds);
        c.auth_lockout_seconds = env_u64("AUTH_LOCKOUT_SECONDS", c.auth_lockout_seconds);
        c.auth_cookie_name = env_string("AUTH_COOKIE_NAME", &c.auth_cookie_name);
        c.auth_cookie_same_site = env_string("AUTH_COOKIE_SAME_SITE", &c.auth_cookie_same_site);
        c.auth_cookie_secure = env_bool("AUTH_COOKIE_SECURE", c.auth_cookie_secure);
        c.auth_trusted_proxy_cidrs =
            env_csv("AUTH_TRUSTED_PROXY_CIDRS", c.auth_trusted_proxy_cidrs);
        c.auth_reset_notifier =
            env_string("AUTH_RESET_NOTIFIER", &c.auth_reset_notifier).to_ascii_lowercase();
        c.public_web_base_url = env_var("PUBLIC_WEB_BASE_URL").or(c.public_web_base_url);

        c.snapshot_ttl_seconds = env_u64("SNAPSHOT_TTL_SECONDS", c.snapshot_ttl_seconds);
        c.maintenance_interval_seconds = env_u64(
            "MAINTENANCE_INTERVAL_SECONDS",
            c.maintenance_interval_seconds,
        );
        c.graceful_shutdown_seconds =
            env_u64("GRACEFUL_SHUTDOWN_SECONDS", c.graceful_shutdown_seconds);
        c.retention_entries = env_usize("LIFETRACE_RETENTION_ENTRIES", c.retention_entries);
        c.beecount_attachment_max_upload_bytes = env_usize(
            "BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES",
            c.beecount_attachment_max_upload_bytes,
        );
        c.file_object_storage_endpoint = env_var("FILE_OBJECT_STORAGE_ENDPOINT");
        c.file_object_storage_bucket = env_var("FILE_OBJECT_STORAGE_BUCKET");
        c.file_object_storage_region =
            env_string("FILE_OBJECT_STORAGE_REGION", &c.file_object_storage_region);
        c.file_object_storage_access_key_id = env_var("FILE_OBJECT_STORAGE_ACCESS_KEY_ID");
        c.file_object_storage_secret_access_key =
            env_var("FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY");
        c.file_object_storage_presign_ttl_seconds = env_usize(
            "FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS",
            c.file_object_storage_presign_ttl_seconds as usize,
        )
        .clamp(60, 3600) as u32;
        c.file_max_upload_bytes = env_var("FILE_MAX_UPLOAD_BYTES")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(c.file_max_upload_bytes);

        c.mail_credential_key = env_var("MAIL_CREDENTIAL_KEY");

        c.deepseek_api_key = env_var("DEEPSEEK_API_KEY");
        c.deepseek_base_url = env_string("DEEPSEEK_BASE_URL", &c.deepseek_base_url);
        c.deepseek_model = env_string("DEEPSEEK_MODEL", &c.deepseek_model);

        c.zhipu_api_key = env_var("ZHIPU_API_KEY");
        c.zhipu_base_url = env_string("ZHIPU_BASE_URL", &c.zhipu_base_url);
        c.photo_challenge_model =
            env_string("PHOTO_CHALLENGE_MODEL", &c.photo_challenge_model);
        c.photo_challenge_access_key = env_var("PHOTO_CHALLENGE_ACCESS_KEY");
        c.photo_challenge_owner_email = env_var("PHOTO_CHALLENGE_OWNER_EMAIL");
        c.photo_staging_ttl_hours = env_var("PHOTO_STAGING_TTL_HOURS")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|hours| *hours > 0)
            .map(|hours| hours.min(24 * 3650));
        c.photo_staging_dir = env_var("PHOTO_STAGING_DIR").unwrap_or_else(|| {
            if c.is_production() {
                "/data/photo-staging".to_owned()
            } else {
                "./data/photo-staging".to_owned()
            }
        });
        c
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.database_path.trim().is_empty() {
            return Err("LIFETRACE_DATABASE_PATH must not be empty".to_owned());
        }
        if !matches!(
            self.auth_registration_mode.as_str(),
            "disabled" | "invite" | "open"
        ) {
            return Err("AUTH_REGISTRATION_MODE must be disabled, invite or open".to_owned());
        }
        if self.auth_password_min_length < 9 || self.auth_password_max_bytes < 64 {
            return Err(
                "AUTH_PASSWORD_MIN_LENGTH must be at least 9 Unicode characters".to_owned(),
            );
        }
        if self.auth_refresh_idle_ttl_seconds > self.auth_refresh_absolute_ttl_seconds {
            return Err("refresh idle TTL must not exceed absolute TTL".to_owned());
        }
        if !(1024..=128 * 1024 * 1024).contains(&self.beecount_attachment_max_upload_bytes) {
            return Err(
                "BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES must be between 1 KiB and 128 MiB".to_owned(),
            );
        }
        if self.file_max_upload_bytes <= 0 {
            return Err("FILE_MAX_UPLOAD_BYTES must be greater than zero".to_owned());
        }
        if self.is_production() {
            if self.dev_auth_enabled {
                return Err("production must not enable DEV_AUTH".to_owned());
            }
            if self.cursor_signing_key.is_none() || self.page_token_signing_key.is_none() {
                return Err(
                    "production requires CURSOR_SIGNING_KEY and PAGE_TOKEN_SIGNING_KEY".to_owned(),
                );
            }
            let password_pepper = self.auth_password_pepper.as_deref().unwrap_or_default();
            let token_pepper = self.auth_token_hash_pepper.as_deref().unwrap_or_default();
            if password_pepper.len() < 32
                || token_pepper.len() < 32
                || password_pepper.starts_with("development-")
                || token_pepper.starts_with("development-")
            {
                return Err("production requires non-default AUTH_PASSWORD_PEPPER and AUTH_TOKEN_HASH_PEPPER of at least 32 characters".to_owned());
            }
            if !self.auth_cookie_secure {
                return Err("production requires AUTH_COOKIE_SECURE=true".to_owned());
            }
            if !self
                .public_web_base_url
                .as_deref()
                .is_some_and(|value| value.starts_with("https://"))
            {
                return Err("production requires HTTPS PUBLIC_WEB_BASE_URL".to_owned());
            }
            for origin in &self.cors_allowed_origins {
                let normalized = origin.trim().to_ascii_lowercase();
                if normalized == "*" || normalized == "null" || !normalized.starts_with("https://") {
                    return Err(format!(
                        "production CORS origin must be an explicit HTTPS origin: {origin}"
                    ));
                }
            }
            if self.auth_reset_notifier == "console" {
                return Err(
                    "production must not use the console password reset notifier".to_owned(),
                );
            }
        }
        Ok(())
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
fn env_string(name: &str, default: &str) -> String {
    env_var(name).unwrap_or_else(|| default.to_owned())
}
fn env_usize(name: &str, default: usize) -> usize {
    env_var(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
fn env_u64(name: &str, default: u64) -> u64 {
    env_var(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
fn env_bool(name: &str, default: bool) -> bool {
    env_var(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}
fn env_csv(name: &str, default: Vec<String>) -> Vec<String> {
    env_var(name)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_config_accepts_default_sqlite_path() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn password_policy_rejects_minimum_below_nine() {
        let config = Config {
            auth_password_min_length: 8,
            ..Config::default()
        };
        assert!(config
            .validate()
            .unwrap_err()
            .contains("at least 9 Unicode characters"));
    }

    #[test]
    fn production_fails_closed_without_auth_secrets() {
        let config = Config {
            environment: "production".to_owned(),
            dev_auth_enabled: false,
            auth_cookie_secure: true,
            public_web_base_url: Some("https://lifetrace.example".to_owned()),
            auth_reset_notifier: "smtp".to_owned(),
            ..Config::default()
        };
        assert!(config.validate().unwrap_err().contains("PEPPER"));
    }

    #[test]
    fn beecount_attachment_limit_is_bounded() {
        let config = Config {
            beecount_attachment_max_upload_bytes: 129 * 1024 * 1024,
            ..Config::default()
        };
        assert!(config
            .validate()
            .unwrap_err()
            .contains("BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES"));
    }
}
