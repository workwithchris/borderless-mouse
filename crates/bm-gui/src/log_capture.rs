use std::sync::Mutex;
use std::sync::Arc;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub struct LogCollector {
    entries: Arc<Mutex<Vec<LogEntry>>>,
    max_entries: usize,
}

impl LogCollector {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn init_as_global_subscriber(&self) {
        let collector = self.entries.clone();
        let max = self.max_entries;
        let layer = LogLayer { entries: collector, max };
        let subscriber = Registry::default().with(layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    }

    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn drain(&self) -> Vec<LogEntry> {
        let mut guard = self.entries.lock().unwrap();
        let result = guard.clone();
        guard.clear();
        result
    }
}

struct LogLayer {
    entries: Arc<Mutex<Vec<LogEntry>>>,
    max: usize,
}

impl<S> Layer<S> for LogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let timestamp = chrono_now();
        let level = format!("{:>5}", event.metadata().level());

        let mut entries = self.entries.lock().unwrap();
        entries.push(LogEntry {
            timestamp,
            level,
            message: visitor.message,
        });
        if entries.len() > self.max {
            entries.remove(0);
        }
    }
}

struct LogVisitor {
    message: String,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, _field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !self.message.is_empty() {
            self.message.push(' ');
        }
        self.message.push_str(&format!("{value:?}"));
    }

    fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
        if !self.message.is_empty() {
            self.message.push(' ');
        }
        self.message.push_str(value);
    }
}

pub(crate) fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
