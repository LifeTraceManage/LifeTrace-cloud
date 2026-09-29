//! Application-wide structured logging built on `tracing`.
//!
//! The project deliberately keeps the collector lightweight: events are emitted
//! through the standard tracing facade and formatted to stdout/stderr so Docker
//! can collect and rotate them. `RUST_LOG` supports a useful subset of
//! EnvFilter syntax, for example:
//!
//! `RUST_LOG=info`
//! `RUST_LOG=warn,lifetrace::http=info,lifetrace::mail=debug`

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{SecondsFormat, Utc};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    fn from_tracing(level: &Level) -> Self {
        if *level == Level::ERROR {
            Self::Error
        } else if *level == Level::WARN {
            Self::Warn
        } else if *level == Level::INFO {
            Self::Info
        } else if *level == Level::DEBUG {
            Self::Debug
        } else {
            Self::Trace
        }
    }

    fn allows(self, level: &Level) -> bool {
        self != Self::Off && self >= Self::from_tracing(level)
    }
}

#[derive(Debug)]
struct LogFilter {
    default: LogLevel,
    directives: Vec<(String, LogLevel)>,
}

impl LogFilter {
    fn from_env() -> Self {
        let raw = std::env::var("RUST_LOG")
            .or_else(|_| std::env::var("LIFETRACE_LOG"))
            .unwrap_or_else(|_| "info".to_owned());
        Self::parse(&raw)
    }

    fn parse(raw: &str) -> Self {
        let mut default = LogLevel::Info;
        let mut directives = Vec::new();
        for directive in raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Some((target, level)) = directive.rsplit_once('=') {
                if let Some(level) = LogLevel::parse(level) {
                    let target = target.trim();
                    if !target.is_empty() {
                        directives.push((target.to_owned(), level));
                    }
                }
            } else if let Some(level) = LogLevel::parse(directive) {
                default = level;
            }
        }
        Self {
            default,
            directives,
        }
    }

    fn level_for(&self, target: &str) -> LogLevel {
        self.directives
            .iter()
            .filter(|(prefix, _)| target.starts_with(prefix))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, level)| *level)
            .unwrap_or(self.default)
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.level_for(metadata.target()).allows(metadata.level())
    }
}

struct CompactSubscriber {
    filter: LogFilter,
    next_span_id: AtomicU64,
}

impl CompactSubscriber {
    fn new() -> Self {
        Self {
            filter: LogFilter::from_env(),
            next_span_id: AtomicU64::new(1),
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    fields: String,
}

impl EventVisitor {
    fn push(&mut self, field: &Field, value: impl std::fmt::Debug) {
        let _ = write!(&mut self.fields, " {}={value:?}", field.name());
    }
}

impl Visit for EventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        } else {
            self.push(field, value);
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}").trim_matches('"').to_owned());
        } else {
            let _ = write!(&mut self.fields, " {}={value:?}", field.name());
        }
    }
}

impl Subscriber for CompactSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.filter.enabled(metadata)
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(self.next_span_id.fetch_add(1, Ordering::Relaxed).max(1))
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        if !self.enabled(event.metadata()) {
            return;
        }
        let metadata = event.metadata();
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let message = visitor.message.as_deref().unwrap_or("event");
        let line = format!(
            "{timestamp} {:<5} [{}] {message}{}",
            metadata.level(),
            metadata.target(),
            visitor.fields
        );
        if *metadata.level() == Level::ERROR || *metadata.level() == Level::WARN {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

/// Install the global tracing subscriber. Calling this twice is harmless for the
/// process: the first subscriber stays active and a diagnostic is emitted.
pub fn init() {
    if let Err(error) = tracing::subscriber::set_global_default(CompactSubscriber::new()) {
        eprintln!("[lifetrace] tracing subscriber already initialized: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_directive_overrides_global_level() {
        let filter = LogFilter::parse("warn,lifetrace::mail=debug");
        assert_eq!(filter.default, LogLevel::Warn);
        assert_eq!(filter.level_for("lifetrace::mail"), LogLevel::Debug);
        assert_eq!(filter.level_for("lifetrace::mail::worker"), LogLevel::Debug);
        assert_eq!(filter.level_for("lifetrace::http"), LogLevel::Warn);
    }
}
