use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

use crate::format::DisplayFormat;

const DETERMINATE_TEMPLATE: &str =
    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len}";
const INDETERMINATE_TEMPLATE: &str = "{spinner:.green} [{elapsed_precise}] {msg}";

/// A progress indicator for a single command invocation.
///
/// In `Console` mode this is an `indicatif` bar (determinate if `total` is
/// known up front, an animated spinner otherwise). In `Json` mode it prints
/// one JSON object per update to stdout instead of drawing anything.
///
/// Finishing (drawing the bar's final frame, or emitting the `"done": true`
/// JSON line) happens automatically on drop, so callers never need to
/// remember to call anything at each return point — including error paths.
pub struct Progress {
    inner: ProgressKind,
}

enum ProgressKind {
    Bar(ProgressBar),
    Json {
        op: &'static str,
        total: Option<u64>,
        current: u64,
        start: Instant,
    },
}

impl Progress {
    pub(crate) fn new(display_format: DisplayFormat, op: &'static str, total: Option<u64>) -> Self {
        let inner = match display_format {
            DisplayFormat::Console => ProgressKind::Bar(console_bar(op, total)),
            DisplayFormat::Json => {
                emit_json(op, 0, total, 0.0, false);
                ProgressKind::Json {
                    op,
                    total,
                    current: 0,
                    start: Instant::now(),
                }
            }
        };
        Progress { inner }
    }

    pub fn inc(&mut self, delta: u64) {
        match &mut self.inner {
            ProgressKind::Bar(bar) => bar.inc(delta),
            ProgressKind::Json {
                op,
                total,
                current,
                start,
            } => {
                *current += delta;
                emit_json(op, *current, *total, start.elapsed().as_secs_f64(), false);
            }
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        match &self.inner {
            ProgressKind::Bar(bar) => bar.finish(),
            ProgressKind::Json {
                op,
                total,
                current,
                start,
            } => emit_json(op, *current, *total, start.elapsed().as_secs_f64(), true),
        }
    }
}

fn console_bar(op: &'static str, total: Option<u64>) -> ProgressBar {
    let bar = match total {
        Some(len) => ProgressBar::new(len),
        None => {
            let bar = ProgressBar::new_spinner();
            bar.enable_steady_tick(Duration::from_millis(100));
            bar
        }
    };

    let template = if total.is_some() {
        DETERMINATE_TEMPLATE
    } else {
        INDETERMINATE_TEMPLATE
    };
    bar.set_style(
        ProgressStyle::with_template(template)
            .unwrap()
            .progress_chars("#>-"),
    );
    bar.set_message(op);
    bar
}

fn emit_json(op: &str, current: u64, total: Option<u64>, elapsed_secs: f64, done: bool) {
    println!(
        "{}",
        serde_json::json!({
            "op": op,
            "current": current,
            "total": total,
            "elapsed_secs": elapsed_secs,
            "done": done,
        })
    );
}
