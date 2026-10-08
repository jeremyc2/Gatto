use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 500;

/// A small, process-local event log intended for troubleshooting. It deliberately
/// never persists data, so tokens and image data cannot be left on disk.
#[derive(Clone)]
pub struct Diagnostics {
    started_at: Instant,
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
}

struct LogEntry {
    elapsed: Duration,
    level: LogLevel,
    message: String,
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Warning,
    Error,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.record(LogLevel::Info, message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.record(LogLevel::Warning, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.record(LogLevel::Error, message);
    }

    pub fn lines(&self) -> Vec<String> {
        let Ok(entries) = self.entries.lock() else {
            return vec!["Diagnostics log is temporarily unavailable.".to_owned()];
        };

        entries
            .iter()
            .map(|entry| {
                let seconds = entry.elapsed.as_secs();
                let milliseconds = entry.elapsed.subsec_millis();
                format!(
                    "+{seconds:04}.{milliseconds:03} {:<5} {}",
                    entry.level.label(),
                    entry.message
                )
            })
            .collect()
    }

    pub fn text(&self) -> String {
        self.lines().join("\n")
    }

    fn record(&self, level: LogLevel, message: impl Into<String>) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if entries.len() == MAX_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(LogEntry {
            elapsed: self.started_at.elapsed(),
            level,
            message: sanitize(message.into()),
        });
    }
}

impl LogLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// GitHub CLI does not normally print tokens, but redact their recognizable
/// prefixes defensively before any text reaches the visible log.
fn sanitize(message: String) -> String {
    ["github_pat_", "ghp_", "gho_", "ghs_", "ghu_"]
        .into_iter()
        .fold(message, redact_tokens_with_prefix)
}

fn redact_tokens_with_prefix(message: String, prefix: &str) -> String {
    let mut redacted = String::with_capacity(message.len());
    let mut remainder = message.as_str();

    while let Some(prefix_index) = remainder.find(prefix) {
        let (before_prefix, token_start) = remainder.split_at(prefix_index);
        redacted.push_str(before_prefix);
        let token_length = token_start
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            .count();
        redacted.push_str("[REDACTED]");
        remainder = &token_start[token_length..];
    }

    redacted.push_str(remainder);
    redacted
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn redacts_github_tokens_embedded_in_diagnostic_text() {
        assert_eq!(
            sanitize("GitHub CLI said token=ghp_example-token_123.".to_owned()),
            "GitHub CLI said token=[REDACTED]."
        );
        assert_eq!(
            sanitize("github_pat_example-token_123 was rejected".to_owned()),
            "[REDACTED] was rejected"
        );
    }
}
