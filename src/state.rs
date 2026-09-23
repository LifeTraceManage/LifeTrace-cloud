//! Shared application state.

use std::sync::Arc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::auth::{AuthProvider, AuthService, DatabaseAuthProvider, DevelopmentAuthProvider};
use crate::beecount::realtime::BeeCountRealtimeHub;
use crate::config::Config;
use crate::repository::sqlite::SqliteRepository;
use crate::repository::SyncRepository;
use crate::sync::cursor_codec::CursorCodec;
use crate::sync::page_token::PageTokenCodec;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("SQLite connection failed: {0}")]
    Pool(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Cloneable shared state for handlers.
#[derive(Clone)]
pub struct AppState {
    /// Embedded SQLite pool shared by all persistent subsystems.
    pub pool: SqlitePool,
    pub store: Arc<dyn SyncRepository>,
    pub config: Arc<Config>,
    pub auth: Arc<dyn AuthProvider>,
    pub auth_service: Arc<AuthService>,
    pub cursor_codec: Arc<CursorCodec>,
    pub page_token_codec: Arc<PageTokenCodec>,
    /// Transitional test compatibility marker. The external BeeCount adapter implementation is retired.
    #[doc(hidden)]
    pub beecount_adapter: Option<()>,
    pub beecount_realtime: Arc<BeeCountRealtimeHub>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let cursor_codec = CursorCodec::new(
            config
                .cursor_signing_key
                .clone()
                .unwrap_or_else(|| "dev-cursor-key".to_owned()),
        );
        let page_token_codec = PageTokenCodec::new(
            config
                .page_token_signing_key
                .clone()
                .unwrap_or_else(|| "dev-page-token-key".to_owned()),
        );

        if let Some(parent) = std::path::Path::new(&config.database_path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let connect_options = SqliteConnectOptions::new()
            .filename(&config.database_path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_lazy_with(connect_options);

        let store: Arc<dyn SyncRepository> = Arc::new(SqliteRepository::new(
            pool.clone(),
            config.clone(),
            cursor_codec.clone(),
            page_token_codec.clone(),
        ));

        let auth_service = Arc::new(AuthService::new(pool.clone(), config.clone()));
        let auth: Arc<dyn AuthProvider> = if config.dev_auth_enabled {
            Arc::new(DevelopmentAuthProvider::new(
                true,
                config.dev_auth_token.clone(),
                lifetrace_contracts::UserId::new(config.dev_auth_user_id.clone()),
                config.dev_auth_device_id.clone(),
            ))
        } else {
            Arc::new(DatabaseAuthProvider::new(
                pool.clone(),
                auth_service.token_manager(),
            ))
        };

        Self {
            pool,
            store,
            config: Arc::new(config),
            auth,
            auth_service,
            cursor_codec: Arc::new(cursor_codec),
            page_token_codec: Arc::new(page_token_codec),
            beecount_adapter: None,
            beecount_realtime: Arc::new(BeeCountRealtimeHub::default()),
        }
    }

    /// Open SQLite and execute the compact baseline migration before serving traffic.
    pub async fn initialize(&self) -> Result<(), StartupError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        if self.config.migration_on_startup {
            sqlx::migrate!().run(&self.pool).await?;
        }
        Ok(())
    }
}
