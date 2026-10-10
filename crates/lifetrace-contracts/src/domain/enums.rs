//! Extensible string enums.
//!
//! Wire enums are string newtypes, not closed Rust enums, so a value added by
//! a newer server/client can never fail parsing of an entire sync batch.
//! Known values are exposed as associated constants.

use std::borrow::Cow;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

macro_rules! wire_string_enum {
    ($name:ident, $doc:expr, [$($variant:ident => $value:expr),* $(,)?]) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            $(pub const $variant: &'static str = $value;)*

            /// Wrap any string. Unknown values are preserved (forward compatible).
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Ok(Self(raw))
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($name))
            }

            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                let mut schema = String::json_schema(generator);
                if let Some(object) = schema.as_object_mut() {
                    object.insert(
                        "description".to_owned(),
                        serde_json::Value::String(format!(
                            "{} Known values: {}. Unknown values are preserved (forward compatible).",
                            $doc,
                            [$($value),*].join(", ")
                        )),
                    );
                }
                schema
            }
        }

        impl TS for $name {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;

            fn decl() -> String {
                format!("type {} = string;", stringify!($name))
            }

            fn decl_concrete() -> String {
                Self::decl()
            }

            fn name() -> String {
                stringify!($name).to_owned()
            }

            fn visit_dependencies(_visitor: &mut impl ts_rs::TypeVisitor) {}

            fn inline() -> String {
                "string".to_owned()
            }

            fn inline_flattened() -> String {
                Self::inline()
            }

            fn output_path() -> Option<std::path::PathBuf> {
                None
            }
        }
    };
}

wire_string_enum!(ActivityType, "Habit activity type.", [
    DURATION => "duration",
    COUNT => "count",
    COMPLETION => "completion",
    WEEKLY => "weekly",
    CONTROL => "control",
]);

wire_string_enum!(ActivityScheduleType, "Habit schedule type.", [
    DAILY => "daily",
    WEEKLY => "weekly",
    CUSTOM => "custom",
    INTERVAL => "interval",
    MONTHLY => "monthly",
]);

wire_string_enum!(ActivityCheckinMethod, "Habit check-in method.", [
    MANUAL => "manual",
    AUTOMATIC => "automatic",
]);

wire_string_enum!(ActivitySyncSource, "Habit sync source.", [
    FITNESS => "fitness",
]);

wire_string_enum!(ActivityLogStatus, "Habit log status.", [
    COMPLETED => "completed",
    PARTIAL => "partial",
    SKIPPED => "skipped",
]);

wire_string_enum!(NoteType, "Note type.", [
    QUICK => "quick",
    DOCUMENT => "document",
    DAILY => "daily",
    HABIT_LOG => "habit_log",
    WORKOUT_REVIEW => "workout_review",
    EXPENSE_NOTE => "expense_note",
    WEEKLY_REVIEW => "weekly_review",
    MONTHLY_REVIEW => "monthly_review",
]);


wire_string_enum!(WorkoutSource, "Workout source.", [
    MANUAL => "manual",
    XUNJI => "xunji",
]);

wire_string_enum!(WorkoutStatus, "Workout status.", [
    COMPLETED => "completed",
    PARTIAL => "partial",
]);

wire_string_enum!(ImportStatus, "Workout import status.", [
    PENDING => "pending",
    SUCCESS => "success",
    FAILED => "failed",
]);

wire_string_enum!(FileStorageState, "File storage state.", [
    LOCAL_ONLY => "local_only",
    PENDING_UPLOAD => "pending_upload",
    SERVER_STORED => "server_stored",
]);

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn all_required_domain_enums_are_present() {
        assert_eq!(ActivityLogStatus::PARTIAL, "partial");
        assert_eq!(NoteType::WEEKLY_REVIEW, "weekly_review");
        assert_eq!(FileStorageState::LOCAL_ONLY, "local_only");
    }
}
