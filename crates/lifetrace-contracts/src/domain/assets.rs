//! LifeTrace Assets wire DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::EntityId;
use crate::time::{LocalDate, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum AssetCategory {
    Phone,
    Tablet,
    Computer,
    Wearable,
    Audio,
    Camera,
    Home,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum AssetStatus {
    Active,
    Idle,
    Lent,
    Repair,
    Sold,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum AssetEventType {
    Purchase,
    UseStart,
    Maintenance,
    Repair,
    Replacement,
    Lend,
    ReturnItem,
    Idle,
    Valuation,
    Sell,
    Retire,
    Note,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Asset {
    pub id: EntityId,
    pub name: String,
    pub brand: String,
    pub model: String,
    pub category: AssetCategory,
    pub status: AssetStatus,
    pub purchase_price: f64,
    pub current_value: f64,
    pub purchase_date: LocalDate,
    pub warranty_until: Option<LocalDate>,
    pub spec: String,
    pub serial_number: String,
    pub location: String,
    pub target_daily_cost: f64,
    pub purchase_channel: String,
    pub maintenance_cost: f64,
    pub recovered_amount: f64,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub server_version: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AssetEvent {
    pub id: EntityId,
    pub asset_id: EntityId,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub event_type: AssetEventType,
    pub date: UtcTimestamp,
    pub title: String,
    pub detail: String,
    pub amount: Option<f64>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub server_version: u64,
}
