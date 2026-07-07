use std::{
    collections::VecDeque,
    fmt,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};

const MAX_LINES: usize = 1_000;

static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

pub fn layer() -> LogBufferLayer {
    LogBufferLayer
}

pub fn recent(limit: usize) -> Vec<String> {
    let logs = LOGS
        .get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_LINES)))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let limit = limit.min(logs.len());

    logs.iter()
        .skip(logs.len().saturating_sub(limit))
        .cloned()
        .collect()
}

pub struct LogBufferLayer;

impl<S> Layer<S> for LogBufferLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = LogVisitor::default();

        event.record(&mut visitor);

        let fields = redact_secrets(&visitor.finish());
        let line = if fields.is_empty() {
            format!(
                "{} {:>5} {}",
                timestamp(),
                metadata.level(),
                metadata.target()
            )
        } else {
            format!(
                "{} {:>5} {} - {}",
                timestamp(),
                metadata.level(),
                metadata.target(),
                fields
            )
        };

        push(line);
    }
}

#[derive(Default)]
struct LogVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl LogVisitor {
    fn finish(self) -> String {
        let mut parts = Vec::new();

        if let Some(message) = self.message {
            parts.push(message);
        }

        parts.extend(self.fields);
        parts.join(" ")
    }
}

impl Visit for LogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = strip_debug_quotes(format!("{value:?}"));

        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }
}

fn push(line: String) {
    let mut logs = LOGS
        .get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_LINES)))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if logs.len() >= MAX_LINES {
        logs.pop_front();
    }

    logs.push_back(line);
}

fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("{seconds}")
}

fn strip_debug_quotes(value: String) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value
    }
}

fn redact_secrets(input: &str) -> String {
    let mut output = input.to_string();

    for key in ["token", "password", "passwd", "secret", "authorization"] {
        output = redact_key(&output, key);
    }

    output
}

fn redact_key(input: &str, key: &str) -> String {
    let mut output = String::new();
    let mut rest = input;

    loop {
        let lower = rest.to_lowercase();

        let Some(pos) = lower.find(key) else {
            output.push_str(rest);
            break;
        };

        output.push_str(&rest[..pos]);

        let after_key = pos + key.len();
        let tail = &rest[after_key..];

        let Some(separator_offset) = tail.find(|ch| ch == '=' || ch == ':') else {
            output.push_str(&rest[pos..after_key]);
            rest = tail;
            continue;
        };

        if separator_offset > 2 {
            output.push_str(&rest[pos..after_key]);
            rest = tail;
            continue;
        }

        let separator_pos = after_key + separator_offset + 1;
        output.push_str(&rest[pos..separator_pos]);
        output.push_str("<redacted>");

        let remaining = &rest[separator_pos..];
        let end = remaining
            .find(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '&')
            .unwrap_or(remaining.len());

        rest = &remaining[end..];
    }

    output
}
