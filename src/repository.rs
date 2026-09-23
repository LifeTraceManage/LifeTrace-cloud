//! Storage abstraction for the sync protocol.
//!
//! SQLite is the single persistence backend for production and tests.

pub mod sqlite;

use async_trait::async_trait;
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, EntitySnapshotV1, PullRequestV1, PullResponseV1, PushRequestV1,
    PushResponseV1, SnapshotRequestV1, SnapshotResponseV1,
};
use lifetrace_contracts::{EntityId, EntityType, UserId};

use crate::error::ApiError;

#[derive(Debug, Clone)]
pub struct StoredEntityRecord {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub server_version: u64,
    pub payload: lifetrace_contracts::json_value::JsonValue,
    pub deleted: bool,
}

#[async_trait]
pub trait SyncRepository: Send + Sync {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, ApiError>;

    async fn push(
        &self,
        user_id: &UserId,
        request: &PushRequestV1,
    ) -> Result<PushResponseV1, ApiError>;

    async fn pull(
        &self,
        user_id: &UserId,
        request: &PullRequestV1,
    ) -> Result<PullResponseV1, ApiError>;

    async fn snapshot(
        &self,
        user_id: &UserId,
        request: &SnapshotRequestV1,
    ) -> Result<SnapshotResponseV1, ApiError>;

    async fn list_entities(
        &self,
        user_id: &UserId,
        entity_type: &str,
    ) -> Result<Vec<EntitySnapshotV1>, ApiError>;

    async fn entity(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<StoredEntityRecord>, ApiError>;

    async fn current_version(
        &self,
        user_id: &UserId,
        entity_type: &str,
        entity_id: &EntityId,
    ) -> Result<Option<u64>, ApiError>;

    async fn change_count(&self, user_id: &UserId) -> Result<usize, ApiError>;
}

