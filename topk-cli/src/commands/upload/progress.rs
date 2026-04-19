use std::sync::atomic::{AtomicU64, Ordering};

use indicatif::{ProgressBar, ProgressStyle};

use crate::output::Output;
use crate::util::plural;

use super::result::UploadError;

/// Reports per-file upload progress. Implementations must be cheap to clone /
/// share across concurrent tasks; the reporter itself owns internal counters.
pub trait ProgressReporter: Send + Sync {
    fn on_upload(&self, ok: bool);
    fn finish(self: Box<Self>, summary: &str, errors: &[UploadError]);
}

/// Build a reporter for the upload phase.
///
/// Falls back to a no-op progress bar when the user asked for JSON output or when
/// stderr is not a TTY (e.g. piping logs to a file). [`Output`] is always stored
/// so per-file errors can be printed consistently in human mode after the bar finishes.
pub fn upload_reporter(total: usize, output: &Output) -> Box<dyn ProgressReporter> {
    let output = *output;
    if !output.can_render_human_stderr() {
        return Box::new(NoopReporter { output });
    }
    Box::new(BarReporter::new(total, output))
}

struct NoopReporter {
    output: Output,
}

impl ProgressReporter for NoopReporter {
    fn on_upload(&self, _ok: bool) {}

    fn finish(self: Box<Self>, _summary: &str, errors: &[UploadError]) {
        report_upload_errors(&self.output, errors);
    }
}

struct BarReporter {
    pb: ProgressBar,
    total: u64,
    completed: AtomicU64,
    failed: AtomicU64,
    output: Output,
}

impl BarReporter {
    fn new(total: usize, output: Output) -> Self {
        let pb = ProgressBar::new(total as u64);
        let style =
            ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg}")
                .map(|s| s.progress_chars("=>-"))
                .unwrap_or_else(|_| ProgressStyle::default_bar());
        pb.set_style(style);
        pb.set_message(format!("0/{total} uploaded"));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Self {
            pb,
            total: total as u64,
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            output,
        }
    }
}

impl ProgressReporter for BarReporter {
    fn on_upload(&self, ok: bool) {
        let completed = self.completed.fetch_add(1, Ordering::Relaxed) + 1;
        let failed = if ok {
            self.failed.load(Ordering::Relaxed)
        } else {
            self.failed.fetch_add(1, Ordering::Relaxed) + 1
        };
        self.pb.set_position(completed);
        let succeeded = completed.saturating_sub(failed);
        let total = self.total;
        if failed == 0 {
            self.pb.set_message(format!("{succeeded}/{total} uploaded"));
        } else {
            self.pb
                .set_message(format!("{succeeded}/{total} uploaded, {failed} failed"));
        }
    }

    fn finish(self: Box<Self>, summary: &str, errors: &[UploadError]) {
        self.pb.finish_with_message(summary.to_string());
        eprintln!();
        report_upload_errors(&self.output, errors);
    }
}

fn report_upload_errors(output: &Output, errors: &[UploadError]) {
    if !output.is_human() {
        return;
    }
    for e in errors {
        let label = e
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| e.doc_id.clone());
        eprintln!("  {label}: {}", e.error);
    }
}

/// Human-friendly summary text for the end of the upload phase.
pub fn summary(total: usize, uploaded: usize, failed: usize) -> String {
    if failed == 0 {
        format!(
            "{uploaded}/{total} {} uploaded",
            plural(total, "file", "files")
        )
    } else {
        format!(
            "{uploaded}/{total} {} uploaded ({failed} failed)",
            plural(total, "file", "files")
        )
    }
}
