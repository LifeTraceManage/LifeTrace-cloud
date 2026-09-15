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
    #[serde(default)]
    pub focus_seconds: Option<u64>,
}

impl DailyReview {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(value) = self.energy {
            if !(1..=5).contains(&value) {
                return Err("energy must be between 1 and 5".to_owned());
            }
        }
        if let Some(value) = self.mood {
            if !(1..=5).contains(&value) {
                return Err("mood must be between 1 and 5".to_owned());
            }
        }
        if let Some(score) = self.completion_score {
            if !score.is_finite() || !(0.0..=1.0).contains(&score) {
                return Err("completionScore must be between 0 and 1".to_owned());
            }
        }
        if let (Some(completed), Some(total)) =
            (self.completed_task_count, self.total_task_count)
        {
            if completed > total {
                return Err("completedTaskCount must not exceed totalTaskCount".to_owned());
            }
        }
        Ok(())
    }
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

impl WeeklyReview {
    pub fn validate(&self) -> Result<(), String> {
        let start = self
            .week_start
            .to_naive()
            .ok_or_else(|| "invalid weekStart".to_owned())?;
        let end = self
            .week_end
            .to_naive()
            .ok_or_else(|| "invalid weekEnd".to_owned())?;
        if end.signed_duration_since(start).num_days() != 6 {
            return Err("weekly review must span exactly 7 calendar days".to_owned());
        }
        if let Some(score) = self.completion_score {
            if !score.is_finite() || !(0.0..=1.0).contains(&score) {
                return Err("completionScore must be between 0 and 1".to_owned());
            }
        }
        if let (Some(completed), Some(total)) =
            (self.completed_task_count, self.total_task_count)
        {
            if completed > total {
                return Err("completedTaskCount must not exceed totalTaskCount".to_owned());
            }
        }
        Ok(())
    }
}
