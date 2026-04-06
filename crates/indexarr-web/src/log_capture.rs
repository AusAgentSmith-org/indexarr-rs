use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::broadcast;

/// A single log entry matching the Python LogEntry format.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: f64,
    pub category: String,
    pub message: String,
    pub level: String,
}

/// Ring-buffer log capture with broadcast for WebSocket subscribers.
pub struct LogCapture {
    buffer: Mutex<VecDeque<LogEntry>>,
    max_lines: usize,
    sender: broadcast::Sender<LogEntry>,
    debug_enabled: Mutex<bool>,
}

impl LogCapture {
    pub fn new(max_lines: usize) -> Arc<Self> {
        let (sender, _) = broadcast::channel(1000);
        Arc::new(Self {
            buffer: Mutex::new(VecDeque::with_capacity(max_lines)),
            max_lines,
            sender,
            debug_enabled: Mutex::new(false),
        })
    }

    /// Add a log entry. Called from the tracing layer.
    pub fn add(&self, message: &str, level: &str) {
        // Parse [tag] prefix
        let (category, msg) = if let Some(rest) = message.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                (rest[..end].to_string(), rest[end + 1..].trim().to_string())
            } else {
                ("general".to_string(), message.to_string())
            }
        } else {
            // Try to extract category from tracing target (e.g., "indexarr_dht::engine")
            ("general".to_string(), message.to_string())
        };

        if level == "debug" && !*self.debug_enabled.lock() {
            return;
        }

        let entry = LogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            category,
            message: msg,
            level: level.to_string(),
        };

        {
            let mut buf = self.buffer.lock();
            if buf.len() >= self.max_lines {
                buf.pop_front();
            }
            buf.push_back(entry.clone());
        }

        // Broadcast to WebSocket subscribers (ignore errors — no subscribers is fine)
        let _ = self.sender.send(entry);
    }

    /// Add a structured log from tracing target + message.
    pub fn add_from_tracing(&self, target: &str, level: &str, message: &str) {
        // Map tracing target to category
        let category = if target.starts_with("indexarr_dht") {
            "dht"
        } else if target.starts_with("indexarr_sync") {
            "sync"
        } else if target.starts_with("indexarr_announcer") {
            "announcer"
        } else if target.starts_with("indexarr_web") {
            "api"
        } else if target.starts_with("indexarr") {
            "system"
        } else if target.starts_with("sqlx") {
            return; // Skip noisy sqlx logs
        } else {
            "general"
        };

        if level == "debug" && !*self.debug_enabled.lock() {
            return;
        }

        let entry = LogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            category: category.to_string(),
            message: message.to_string(),
            level: level.to_string(),
        };

        {
            let mut buf = self.buffer.lock();
            if buf.len() >= self.max_lines {
                buf.pop_front();
            }
            buf.push_back(entry.clone());
        }

        let _ = self.sender.send(entry);
    }

    pub fn get_recent(&self, count: usize, category: Option<&str>) -> Vec<LogEntry> {
        let buf = self.buffer.lock();
        let entries: Vec<LogEntry> = if let Some(cat) = category {
            buf.iter().filter(|e| e.category == cat).cloned().collect()
        } else {
            buf.iter().cloned().collect()
        };
        let start = entries.len().saturating_sub(count);
        entries[start..].to_vec()
    }

    pub fn categories(&self) -> Vec<String> {
        let buf = self.buffer.lock();
        let mut cats: Vec<String> = buf.iter().map(|e| e.category.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
        cats.sort();
        if cats.is_empty() {
            cats = vec!["system".into(), "dht".into(), "resolver".into(), "announcer".into(), "sync".into(), "api".into()];
        }
        cats
    }

    pub fn debug_enabled(&self) -> bool {
        *self.debug_enabled.lock()
    }

    pub fn set_debug_enabled(&self, enabled: bool) {
        *self.debug_enabled.lock() = enabled;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.sender.subscribe()
    }
}

/// Tracing layer that feeds into LogCapture.
pub struct LogCaptureLayer {
    capture: Arc<LogCapture>,
}

impl LogCaptureLayer {
    pub fn new(capture: Arc<LogCapture>) -> Self {
        Self { capture }
    }
}

impl<S> tracing_subscriber::Layer<S> for LogCaptureLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => "error",
            tracing::Level::WARN => "warning",
            tracing::Level::INFO => "info",
            tracing::Level::DEBUG => "debug",
            tracing::Level::TRACE => return, // skip trace
        };

        // Extract the message from the event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let message = if visitor.message.is_empty() {
            // Build from fields
            visitor.fields.join(", ")
        } else if !visitor.fields.is_empty() {
            format!("{} {}", visitor.message, visitor.fields.join(", "))
        } else {
            visitor.message
        };

        if !message.is_empty() {
            self.capture.add_from_tracing(event.metadata().target(), level, &message);
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: Vec<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}").trim_matches('"').to_string();
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.push(format!("{}={}", field.name(), value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.push(format!("{}={}", field.name(), value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields.push(format!("{}={:.2}", field.name(), value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.push(format!("{}={}", field.name(), value));
    }
}
