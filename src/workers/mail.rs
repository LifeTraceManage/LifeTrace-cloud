use std::time::Duration;

use crate::mail::credential::CredentialCipher;
use crate::mail::domain::MailAccountSecret;
use crate::mail::{protocol, MailService};
use crate::AppState;
use lifetrace_contracts::UserId;
use tokio::task::JoinSet;

const IDLE_WINDOW: Duration = Duration::from_secs(55);
const EMPTY_IDLE_SLEEP: Duration = Duration::from_secs(30);
const MAX_IDLE_ACCOUNTS: i64 = 32;
const MAX_POLL_ACCOUNTS: i64 = 100;

pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Fail before entering the worker loop when the external envelope key is absent or malformed.
    let credential_cipher = CredentialCipher::from_config(&state.config)?;

    let service = MailService::new(state.pool.clone(), state.config.clone());
    tracing::info!(
        target: "lifetrace::mail",
        idle_window_seconds = IDLE_WINDOW.as_secs(),
        poll_account_limit = MAX_POLL_ACCOUNTS,
        "mail worker started"
    );

    loop {
        let idle_accounts = match load_idle_accounts(&state, MAX_IDLE_ACCOUNTS).await {
            Ok(accounts) => accounts,
            Err(error) => {
                tracing::error!(
                    target: "lifetrace::mail",
                    error = %error,
                    "mail worker account scan failed"
                );
                tokio::time::sleep(EMPTY_IDLE_SLEEP).await;
                continue;
            }
        };
        let mut idle_tasks = JoinSet::new();
        for account in idle_accounts.iter().cloned() {
            let cipher = credential_cipher.clone();
            idle_tasks.spawn(async move {
                let secret = cipher
                    .decrypt(&account.credential_ciphertext, &account.credential_nonce)
                    .ok()?;
                match protocol::wait_for_inbox_change(account.clone(), secret, IDLE_WINDOW).await {
                    Ok(true) => Some((account.user_id, account.id)),
                    Ok(false) | Err(_) => None,
                }
            });
        }

        while let Some(result) = idle_tasks.join_next().await {
            if let Ok(Some((user_id, account_id))) = result {
                let user = UserId::new(user_id.to_string());
                match service.sync_account_incremental(&user, account_id).await {
                    Ok(synced_messages) => {
                        if synced_messages > 0 {
                            tracing::info!(
                                target: "lifetrace::mail",
                                account_id = %account_id,
                                messages_synced = synced_messages,
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
                                trigger = "idle",
                                "mail idle sync completed without persisted changes"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: "lifetrace::mail",
                            account_id = %account_id,
                            error = %error,
                            trigger = "idle",
                            "mail idle-triggered sync failed"
                        );
                    }
                }
            }
        }

        match service.sync_due_accounts(MAX_POLL_ACCOUNTS).await {
            Ok(stats) if stats.messages_synced > 0 => {
                tracing::info!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    failed = stats.failed,
                    messages_synced = stats.messages_synced,
                    trigger = "poll",
                    "mail polling synchronized changes"
                );
                state.mail_realtime.publish_global_updated("poll");
            }
            Ok(stats) if stats.failed > 0 => {
                tracing::warn!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    failed = stats.failed,
                    trigger = "poll",
                    "mail polling completed with failures"
                );
            }
            Ok(stats) => {
                tracing::debug!(
                    target: "lifetrace::mail",
                    attempted = stats.attempted,
                    succeeded = stats.succeeded,
                    trigger = "poll",
                    "mail polling completed without changes"
                );
            }
            Err(error) => {
                tracing::error!(
                    target: "lifetrace::mail",
                    error = %error,
                    trigger = "poll",
                    "mail polling sync failed"
                );
            }
        }

        if idle_accounts.is_empty() {
            tokio::time::sleep(EMPTY_IDLE_SLEEP).await;
        }
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
