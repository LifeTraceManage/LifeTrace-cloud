use std::time::{Duration, Instant};

use crate::mail::credential::CredentialCipher;
use crate::mail::domain::MailAccountSecret;
use crate::mail::{protocol, MailService};
use crate::AppState;
use lifetrace_contracts::UserId;
use tokio::task::JoinSet;
use uuid::Uuid;

const IDLE_WINDOW: Duration = Duration::from_secs(55);
const IDLE_ROUND_PAUSE: Duration = Duration::from_secs(1);
const EMPTY_IDLE_SLEEP: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_secs(30);
const RECONCILE_INTERVAL: Duration = Duration::from_secs(300);
const MAX_IDLE_ACCOUNTS: i64 = 32;
const MAX_POLL_ACCOUNTS: i64 = 100;

pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Fail before entering the worker loops when the external envelope key is absent or malformed.
    let credential_cipher = CredentialCipher::from_config(&state.config)?;

    tracing::info!(
        target: "lifetrace::mail",
        idle_window_seconds = IDLE_WINDOW.as_secs(),
        poll_interval_seconds = POLL_INTERVAL.as_secs(),
        reconcile_interval_seconds = RECONCILE_INTERVAL.as_secs(),
        poll_account_limit = MAX_POLL_ACCOUNTS,
        "mail worker started"
    );

    let idle = run_idle_loop(state.clone(), credential_cipher);
    let poll = run_poll_loop(state.clone());
    let reconcile = run_reconcile_loop(state);
    tokio::try_join!(idle, poll, reconcile)?;
    Ok(())
}

async fn run_idle_loop(
    state: AppState,
    credential_cipher: CredentialCipher,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = MailService::new(state.pool.clone(), state.config.clone());

    loop {
        let idle_accounts = match load_idle_accounts(&state, MAX_IDLE_ACCOUNTS).await {
            Ok(accounts) => accounts,
            Err(error) => {
                tracing::error!(
                    target: "lifetrace::mail",
                    error = %error,
                    trigger = "idle",
                    "mail idle account scan failed"
                );
                tokio::time::sleep(EMPTY_IDLE_SLEEP).await;
                continue;
            }
        };

        if idle_accounts.is_empty() {
            tokio::time::sleep(EMPTY_IDLE_SLEEP).await;
            continue;
        }

        let mut idle_tasks = JoinSet::new();
        for account in idle_accounts {
            let cipher = credential_cipher.clone();
            idle_tasks.spawn(async move {
                let result = async {
                    let secret = cipher
                        .decrypt(&account.credential_ciphertext, &account.credential_nonce)
                        .map_err(|error| error.to_string())?;
                    protocol::wait_for_inbox_change(account.clone(), secret, IDLE_WINDOW)
                        .await
                        .map_err(|error| error.to_string())
                }
                .await;
                (account.user_id, account.id, result)
            });
        }

        while let Some(result) = idle_tasks.join_next().await {
            let (user_id, account_id, changed) = match result {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(
                        target: "lifetrace::mail",
                        error = %error,
                        trigger = "idle",
                        "mail idle task failed"
                    );
                    continue;
                }
            };

            match changed {
                Ok(true) => {
                    let user = UserId::new(user_id.to_string());
                    let started = Instant::now();
                    tracing::debug!(
                        target: "lifetrace::mail",
                        account_id = %account_id,
                        trigger = "idle",
                        "mail inbox change detected"
                    );
                    match service
                        .sync_folder_role_incremental(&user, account_id, "inbox")
                        .await
                    {
                        Ok(synced_messages) => {
                            let duration_ms = started.elapsed().as_millis() as u64;
                            if synced_messages > 0 {
                                tracing::info!(
                                    target: "lifetrace::mail",
                                    account_id = %account_id,
                                    messages_synced = synced_messages,
                                    duration_ms,
                                    trigger = "idle",
                                    "mail inbox changes synchronized"
                                );
                                state.mail_realtime.publish_account_updated(
                                    user.as_str(),
                                    account_id,
                                    synced_messages,
                                    "idle",
                                );
                            } else {
                                tracing::debug!(
                                    target: "lifetrace::mail",
                                    account_id = %account_id,
                                    duration_ms,
                                    trigger = "idle",
                                    "mail idle sync completed without persisted changes"
                                );
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: "lifetrace::mail",
                                account_id = %account_id,
                                duration_ms = started.elapsed().as_millis() as u64,
                                error = %error,
                                trigger = "idle",
                                "mail idle-triggered inbox sync failed"
                            );
                        }
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(
                        target: "lifetrace::mail",
                        account_id = %account_id,
                        error = %error,
                        trigger = "idle",
                        "mail idle wait failed; retrying"
                    );
                }
            }
        }

        // Prevent a provider/network failure from turning the IDLE supervisor into a hot loop.
        tokio::time::sleep(IDLE_ROUND_PAUSE).await;
    }
}

async fn run_poll_loop(
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = MailService::new(state.pool.clone(), state.config.clone());

    loop {
        match load_poll_accounts(&state, MAX_POLL_ACCOUNTS).await {
            Ok(accounts) => {
                for (user_id, account_id) in accounts {
                    let user = UserId::new(user_id.to_string());
                    let started = Instant::now();
                    match service
                        .sync_folder_role_incremental(&user, account_id, "inbox")
                        .await
                    {
                        Ok(synced_messages) => {
                            let duration_ms = started.elapsed().as_millis() as u64;
                            if synced_messages > 0 {
                                tracing::info!(
                                    target: "lifetrace::mail",
                                    account_id = %account_id,
                                    messages_synced = synced_messages,
                                    duration_ms,
                                    trigger = "poll",
                                    "mail inbox polling synchronized changes"
                                );
                                state.mail_realtime.publish_account_updated(
                                    user.as_str(),
                                    account_id,
                                    synced_messages,
                                    "poll",
                                );
                            } else {
                                tracing::debug!(
                                    target: "lifetrace::mail",
                                    account_id = %account_id,
                                    duration_ms,
                                    trigger = "poll",
                                    "mail inbox polling completed without changes"
                                );
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: "lifetrace::mail",
                                account_id = %account_id,
                                duration_ms = started.elapsed().as_millis() as u64,
                                error = %error,
                                trigger = "poll",
                                "mail inbox polling sync failed"
                            );
                        }
                    }
                }
            }
            Err(error) => {
                tracing::error!(
                    target: "lifetrace::mail",
                    error = %error,
                    trigger = "poll",
                    "mail polling account scan failed"
                );
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn run_reconcile_loop(
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = MailService::new(state.pool.clone(), state.config.clone());

    // Real-time inbox paths get priority at startup. Full-folder reconciliation is low frequency.
    tokio::time::sleep(RECONCILE_INTERVAL).await;

    loop {
        match service.sync_due_accounts(MAX_POLL_ACCOUNTS).await {
            Ok(stats) if stats.messages_synced > 0 => {
                tracing::info!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    failed = stats.failed,
                    messages_synced = stats.messages_synced,
                    trigger = "reconcile",
                    "mail full-folder reconciliation synchronized changes"
                );
                state.mail_realtime.publish_global_updated("reconcile");
            }
            Ok(stats) if stats.failed > 0 => {
                tracing::warn!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    failed = stats.failed,
                    trigger = "reconcile",
                    "mail full-folder reconciliation completed with failures"
                );
            }
            Ok(stats) => {
                tracing::debug!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    trigger = "reconcile",
                    "mail full-folder reconciliation completed without changes"
                );
            }
            Err(error) => {
                tracing::error!(
                    target: "lifetrace::mail",
                    error = %error,
                    trigger = "reconcile",
                    "mail full-folder reconciliation failed"
                );
            }
        }

        tokio::time::sleep(RECONCILE_INTERVAL).await;
    }
}

async fn load_idle_accounts(
    state: &AppState,
    limit: i64,
) -> Result<Vec<MailAccountSecret>, sqlx::Error> {
    sqlx::query_as::<_, MailAccountSecret>(
        r#"
        SELECT id,user_id,provider,email_address,display_name,
               imap_host,imap_port,imap_security,smtp_host,smtp_port,smtp_security,
               username,credential_ciphertext,credential_nonce,status
        FROM mail_accounts
        WHERE deleted_at IS NULL AND status IN ('active','degraded') AND idle_supported=TRUE
        ORDER BY last_sync_at NULLS FIRST
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(&state.pool)
    .await
}

async fn load_poll_accounts(
    state: &AppState,
    limit: i64,
) -> Result<Vec<(Uuid, Uuid)>, sqlx::Error> {
    sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT a.user_id,a.id
        FROM mail_accounts a
        JOIN mail_folders f
          ON f.user_id=a.user_id
         AND f.account_id=a.id
         AND f.normalized_role='inbox'
         AND f.sync_enabled=TRUE
        WHERE a.deleted_at IS NULL
          AND a.status IN ('active','degraded')
          AND a.idle_supported=FALSE
          AND (f.last_sync_at IS NULL OR f.last_sync_at < datetime('now','-30 seconds'))
        ORDER BY f.last_sync_at NULLS FIRST
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(&state.pool)
    .await
}
