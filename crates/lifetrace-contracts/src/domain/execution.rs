//! Execute Android wire DTOs that do not use the desktop `EntityMeta` envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::ids::{EntityId, UserId};
use crate::time::{LocalDate, UtcTimestamp};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// `execution.goal`, matching the shared Goal layer used by LifeTrace desktop/web.
///
/// Goal is the layer above projects: Goal -> Project -> Task. The wire contract
/// intentionally preserves the existing nullable presentation fields and
/// timestamp fields so older clients can continue syncing the same entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExecutionGoal {
    pub meta: EntityMeta,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: GoalStatus,
    #[serde(default)]
    pub target_at: Option<UtcTimestamp>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub completed_at: Option<UtcTimestamp>,
}

impl ExecutionGoal {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("goal name must not be empty".to_owned());
        }
        Ok(())
    }
}


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

    /// Solar source date. For lunar entries this remains the legacy Android
    /// compatibility anchor; lunar fields below are the source of truth.
    pub date: LocalDate,
    pub repeat: ImportantDateRepeat,
    pub kind: ImportantDateKind,
    pub calendar: ImportantDateCalendar,

    /// Required for one-off lunar dates and optional for yearly lunar dates.
    #[serde(default)]
    pub lunar_year: Option<i32>,
    pub lunar_month: Option<u8>,
    pub lunar_day: Option<u8>,
    pub lunar_leap_month: bool,

    /// Existing payloads predate this field, so omission keeps them enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl ImportantDate {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("important date title must not be empty".to_owned());
        }

        match self.calendar {
            ImportantDateCalendar::Solar => {
                if self.lunar_year.is_some()
                    || self.lunar_month.is_some()
                    || self.lunar_day.is_some()
                    || self.lunar_leap_month
                {
                    return Err(
                        "solar important date must not contain lunar source fields".to_owned(),
                    );
                }
            }
            ImportantDateCalendar::Lunar => {
                let month = self
                    .lunar_month
                    .ok_or_else(|| "lunarMonth is required for lunar date".to_owned())?;
                let day = self
                    .lunar_day
                    .ok_or_else(|| "lunarDay is required for lunar date".to_owned())?;
                if !(1..=12).contains(&month) {
                    return Err("lunarMonth must be between 1 and 12".to_owned());
                }
                if !(1..=30).contains(&day) {
                    return Err("lunarDay must be between 1 and 30".to_owned());
                }
                if self.repeat == ImportantDateRepeat::Once && self.lunar_year.is_none() {
                    return Err("lunarYear is required for one-off lunar date".to_owned());
                }
                if let Some(year) = self.lunar_year {
                    if !(1900..=2199).contains(&year) {
                        return Err("lunarYear must be between 1900 and 2199".to_owned());
                    }
                }
            }
        }

        Ok(())
    }
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
/// `fired`. Clients use the subject fields to route notification taps.
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
