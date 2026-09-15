//! Review DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::time::LocalDate;

/// `review.daily`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DailyReview {
    pub meta: EntityMeta,
    pub review_date: LocalDate,
    pub energy: Option<i64>,
    pub mood: Option<i64>,
    pub completion_score: Option<f64>,
    pub best_thing: Option<String>,
    pub problem: Option<String>,
    pub tomorrow_priority: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub completed_task_count: Option<u64>,
    #[serde(default)]
    pub total_task_count: Option<u64>,
}


/// `execution.weekly_review`.
///
/// Week boundaries are explicit local dates so clients can render and query
/// historical reviews without depending on locale-specific week numbering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WeeklyReview {
    pub meta: EntityMeta,
    pub week_start: LocalDate,
    pub week_end: LocalDate,

    #[serde(default)]
    pub completion_score: Option<f64>,
    #[serde(default)]
    pub completed_task_count: Option<u64>,
    #[serde(default)]
    pub total_task_count: Option<u64>,
    #[serde(default)]
    pub focus_seconds: Option<u64>,

    #[serde(default)]
    pub completion_summary: Option<String>,
    #[serde(default)]
    pub best_thing: Option<String>,
    #[serde(default)]
    pub problem: Option<String>,
    #[serde(default)]
    pub improvement: Option<String>,
    #[serde(default)]
    pub next_week_priority: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}
