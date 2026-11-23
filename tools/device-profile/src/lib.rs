//! Opt-in collector for the `unbill::profile` tracing target.
//!
//! Install this layer in an embedding application to observe the same device
//! spans without adding an RPC transport. Events contain static operation names
//! and numeric timings only. Closed spans are streamed to disk as JSONL.
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::ThreadId,
    time::Instant,
};
use tracing::{
    Id, Subscriber,
    field::{Field, Visit},
    span::Attributes,
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

pub const TARGET: &str = "unbill::profile";

struct Timing {
    id: u64,
    parent: Option<u64>,
    start: Instant,
    operation: Option<String>,
    // Entered time is inclusive elapsed thread occupancy, NOT CPU time.
    entered: HashMap<ThreadId, Vec<Instant>>,
    entered_us: u64,
}

#[derive(Default)]
struct Operation(Option<String>);
impl Visit for Operation {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "operation" {
            self.0 = Some(value.to_owned());
        }
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

struct Output {
    file: File,
    error: Option<io::Error>,
}

/// Finite-memory collector: holds only live spans, never the entire recording.
#[derive(Clone)]
pub struct Capture {
    start: Instant,
    next_id: Arc<AtomicU64>,
    output: Arc<Mutex<Output>>,
}
impl Capture {
    /// Refuses to overwrite an existing capture.
    pub fn create(path: &Path) -> io::Result<Self> {
        Ok(Self {
            start: Instant::now(),
            next_id: Arc::new(AtomicU64::new(1)),
            output: Arc::new(Mutex::new(Output {
                file: File::options().write(true).create_new(true).open(path)?,
                error: None,
            })),
        })
    }
    /// Time since capture creation, for resource samples sharing the trace clock.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Surface write failures that cannot be returned from tracing callbacks.
    pub fn finish(&self) -> io::Result<()> {
        let mut out = self
            .output
            .lock()
            .map_err(|_| io::Error::other("capture poisoned"))?;
        if let Some(error) = out.error.take() {
            return Err(error);
        }
        out.file.flush()
    }
}
impl<S> Layer<S> for Capture
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().target() != TARGET {
            return;
        }
        let Some(span) = ctx.span(id) else { return };
        let parent = span
            .parent()
            .and_then(|p| p.extensions().get::<Timing>().map(|t| t.id));
        let mut operation = Operation::default();
        attrs.record(&mut operation);
        span.extensions_mut().insert(Timing {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            parent,
            start: Instant::now(),
            operation: operation.0,
            entered: HashMap::new(),
            entered_us: 0,
        });
    }
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(t) = span.extensions_mut().get_mut::<Timing>()
        {
            t.entered
                .entry(std::thread::current().id())
                .or_default()
                .push(Instant::now());
        }
    }
    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(t) = span.extensions_mut().get_mut::<Timing>()
            && let Some(stack) = t.entered.get_mut(&std::thread::current().id())
            && let Some(start) = stack.pop()
        {
            t.entered_us = t.entered_us.saturating_add(micros(start.elapsed()));
        }
    }
    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let ext = span.extensions();
        let Some(t) = ext.get::<Timing>() else { return };
        let row = serde_json::json!({
            "schema": 1, "id": t.id, "parent": t.parent,
            "name": span.name(), "operation": t.operation,
            "start_us": micros(t.start.duration_since(self.start)),
            "duration_us": micros(t.start.elapsed()), "entered_us": t.entered_us,
        });
        if let Ok(mut out) = self.output.lock() {
            if out.error.is_some() {
                return;
            }
            let result = serde_json::to_writer(&mut out.file, &row)
                .map_err(io::Error::other)
                .and_then(|()| out.file.write_all(b"\n"));
            if let Err(error) = result {
                out.error = Some(error);
            }
        }
    }
}
fn micros(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::{filter::filter_fn, prelude::*};

    #[test]
    fn captures_parentage_and_never_records_secret_fields() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("unbill-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("spans.jsonl");
        let capture = Capture::create(&path)?;
        let subscriber = tracing_subscriber::registry().with(
            capture
                .clone()
                .with_filter(filter_fn(|m| m.target() == TARGET)),
        );
        tracing::subscriber::with_default(subscriber, || {
            let request = tracing::trace_span!(target: "unbill::profile", "request", password = "never-record-this");
            let _enter = request.enter();
            {
                let _child =
                    tracing::trace_span!(target: "unbill::profile", "child", operation = "read")
                        .entered();
            }
        });
        capture.finish()?;
        let text = std::fs::read_to_string(&path)?;
        assert!(!text.contains("never-record-this"));
        let rows: Vec<serde_json::Value> = text
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["parent"], rows[1]["id"]);
        assert_eq!(rows[0]["operation"], "read");
        assert!(Capture::create(&path).is_err());
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }
}
