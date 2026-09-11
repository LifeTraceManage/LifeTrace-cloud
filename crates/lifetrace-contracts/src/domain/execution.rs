//! Execute Android wire DTOs that do not use the desktop `EntityMeta` envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::ids::{EntityId, UserId};
use crate::time::{LocalDate, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateRepeat {
    Once,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateKind {
    Birthday,
    Anniversary,
    Milestone,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateCalendar {
    Solar,
    Lunar,
}

/// `execution.important_date`, matching `ImportantDateWireMapper` on Android.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ImportantDate {
    pub id: EntityId,
    pub user_id: UserId,
    pub title: String,
    pub date: LocalDate,
    pub repeat: ImportantDateRepeat,
    pub kind: ImportantDateKind,
    pub calendar: ImportantDateCalendar,
    pub lunar_month: Option<u8>,
    pub lunar_day: Option<u8>,
    pub lunar_leap_month: bool,
}


/// Reminder delivery lifecycle shared by Cloud and clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ReminderStatus {
    Scheduled,
    Fired,
    Dismissed,
    Cancelled,
}

/// `execution.reminder`.
///
/// This intentionally preserves the existing Execute wire shape
/// (`subjectType`, `subjectId`, `fireKey`) while making it a strict typed
/// contract. The Cloud execution worker advances due `scheduled` reminders to
/// `fired`. Clients use the subject fields to route notification taps, while
/// `fireKey` remains the compatibility/idempotency key used by existing Execute data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Reminder {
    pub meta: EntityMeta,
    pub subject_type: String,
    pub subject_id: EntityId,
    pub trigger_at: UtcTimestamp,
    pub status: ReminderStatus,
    pub fire_key: String,
    pub snoozed_until: Option<UtcTimestamp>,
    pub last_fired_at: Option<UtcTimestamp>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum FocusMode {
    Short,
    Long,
}

/// `execution.focus_session`, matching `FocusSessionWireMapper` on Android.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: EntityId,
    pub user_id: UserId,
    pub task_id: Option<EntityId>,
    pub mode: FocusMode,
    pub started_at: UtcTimestamp,
    pub ended_at: UtcTimestamp,
    pub focus_seconds: u64,
    pub completed: bool,
}
