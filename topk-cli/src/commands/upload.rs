use std::collections::HashMap;
use std::fmt;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytesize::ByteSize;
use clap::Args;
use colored::Colorize;
use futures::stream::{self, StreamExt, TryStreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use topk_rs::{
    proto::v1::{ctx::file::InputFile, data::Value},
    Client, Error,
};

use crate::output::{Output, OutputFormat};
use crate::util::{
    files::{resolve_files, UploadFile},
    parse_seconds, plural,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadError {
    pub doc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub total: usize,
    pub uploaded: usize,
    pub errors: Vec<UploadError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed: Option<bool>,
}

impl fmt::Display for UploadResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.total == 0 {
            return f.write_str("No files found for upload.");
        }

        if self.uploaded == 0 {
            return f.write_str("Upload skipped.");
        }

        match self.processed {
            Some(true) => write!(
                f,
                "Uploaded and processed {} {}.",
                self.uploaded,
                plural(self.uploaded, "file", "files")
            ),
            Some(false) | None => write!(
                f,
                "Uploaded {} {}.",
                self.uploaded,
                plural(self.uploaded, "file", "files")
            ),
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct UploadArgs {
    /// Dataset to upload into
    #[arg(short = 'd', long, value_name = "DATASET_NAME")]
    pub dataset: String,
    /// Recurse into subdirectories when any PATTERN is a directory
    #[arg(short = 'r', long)]
    pub recursive: bool,
    /// Number of concurrent uploads (1–64)
    #[arg(short = 'c', long, default_value = "32", value_parser = clap::value_parser!(u64).range(1..=64))]
    pub concurrency: u64,
    /// Skip upload confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
    /// Preview files without uploading
    #[arg(long)]
    pub dry_run: bool,
    /// Wait for all uploaded files to be fully processed
    #[arg(short = 'w', long)]
    pub wait: bool,
    /// Timeout for uploading files in seconds (default: 30 minutes)
    #[arg(long, value_name = "SECS", default_value = "1800", value_parser = parse_seconds)]
    pub timeout: Duration,
    /// File paths, directories, or glob patterns (e.g. "./report.pdf" "./docs" "*.pdf" "docs/**/*.md")
    #[arg(value_name = "PATTERN", required = true, num_args = 1..)]
    pub patterns: Vec<String>,
}

fn make_upload_plan(
    cwd: &Path,
    patterns: &[String],
    recursive: bool,
) -> Result<Vec<UploadFile>, Error> {
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for pattern in patterns {
        for file in resolve_files(cwd, pattern, recursive)? {
            if seen.insert(file.doc_id.clone()) {
                files.push(file);
            }
        }
    }

    Ok(files)
}

pub(crate) fn doc_id_from_path(path: &Path) -> Result<String, Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            path.canonicalize()
                .map_err(Error::IoError)?
                .to_string_lossy()
                .as_bytes()
        )
    ))
}

trait ProgressReporter: Send + Sync {
    fn on_upload(&self, ok: bool);
    fn finish(self: Box<Self>, total: usize, files: &[FileUpload]);
}

fn upload_reporter(total: usize, output: &Output) -> Box<dyn ProgressReporter> {
    let output = *output;
    if matches!(output.format, OutputFormat::Json) || !std::io::stderr().is_terminal() {
        return Box::new(NoopReporter { output });
    }
    Box::new(BarReporter::new(total, output))
}

fn progress_summary(total: usize, uploaded: usize, failed: usize) -> String {
    let files = plural(total, "file", "files");
    match failed {
        0 => format!("{uploaded}/{total} {files} uploaded"),
        _ => format!("{uploaded}/{total} {files} uploaded ({failed} failed)"),
    }
}

struct NoopReporter {
    output: Output,
}

impl ProgressReporter for NoopReporter {
    fn on_upload(&self, _ok: bool) {}

    fn finish(self: Box<Self>, _total: usize, files: &[FileUpload]) {
        report_upload_errors(&self.output, files);
    }
}

struct BarReporter {
    progress_bar: ProgressBar,
    total: u64,
    completed: AtomicU64,
    failed: AtomicU64,
    output: Output,
}

impl BarReporter {
    fn new(total: usize, output: Output) -> Self {
        let progress_bar = ProgressBar::new(total as u64);
        let style =
            ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg}")
                .map(|s| s.progress_chars("=>-"))
                .unwrap_or_else(|_| ProgressStyle::default_bar());
        progress_bar.set_style(style);
        progress_bar.set_message(format!("0/{total} uploaded"));
        progress_bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self {
            progress_bar,
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
        let succeeded = completed.saturating_sub(failed);

        let msg = match failed {
            0 => format!("{succeeded}/{} uploaded", self.total),
            _ => format!("{succeeded}/{} uploaded, {failed} failed", self.total),
        };

        self.progress_bar.set_position(completed);
        self.progress_bar.set_message(msg);
    }

    fn finish(self: Box<Self>, total: usize, files: &[FileUpload]) {
        let uploaded = files.iter().filter(|f| f.result.is_ok()).count();
        let failed = files.iter().filter(|f| f.result.is_err()).count();
        self.progress_bar
            .finish_with_message(progress_summary(total, uploaded, failed));
        report_upload_errors(&self.output, files);
    }
}

fn report_upload_errors(output: &Output, files: &[FileUpload]) {
    for f in files {
        if let Err(e) = &f.result {
            output.warn(&format!("  {}: {e}", f.path.display()));
        }
    }
}

struct FileUpload {
    doc_id: String,
    path: PathBuf,
    result: Result<String, Error>,
}

async fn upload_all(
    client: &Client,
    dataset: &str,
    files: Vec<UploadFile>,
    concurrency: usize,
    reporter: &dyn ProgressReporter,
) -> Vec<FileUpload> {
    stream::iter(files)
        .map(|file| {
            let client = client.clone();
            let dataset = dataset.to_string();
            async move {
                let result = async {
                    let handle = client
                        .dataset(&dataset)
                        .upsert_file(
                            file.doc_id.clone(),
                            InputFile::from_path(&file.path)?,
                            HashMap::<String, Value>::default(),
                        )
                        .await?
                        .into_inner()
                        .handle;
                    Ok::<String, Error>(handle)
                }
                .await;
                reporter.on_upload(result.is_ok());
                FileUpload {
                    doc_id: file.doc_id,
                    path: file.path,
                    result,
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

async fn wait_for_all(
    client: &Client,
    dataset: &str,
    files: &[FileUpload],
    concurrency: usize,
    output: &Output,
) -> Result<bool, Error> {
    let handles: Vec<String> = files
        .iter()
        .filter_map(|f| f.result.as_ref().ok().cloned())
        .collect();
    let total = handles.len() as u64;
    let spinner = output.spinner(format!("0/{total} processed — press Enter to skip"));
    let progress_bar = spinner.progress_bar.clone();

    let process_fut = {
        let client = client.clone();
        let dataset = dataset.to_string();
        async move {
            let done = Arc::new(AtomicU64::new(0));
            stream::iter(handles)
                .map(|handle| {
                    let client = client.clone();
                    let dataset = dataset.to_string();
                    let progress_bar = progress_bar.clone();
                    let done = done.clone();
                    async move {
                        client
                            .dataset(&dataset)
                            .wait_for_handle(&handle, None)
                            .await?;
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(pb) = &progress_bar {
                            pb.set_message(format!("{n}/{total} processed — press Enter to skip"));
                        }
                        Ok::<_, Error>(())
                    }
                })
                .buffer_unordered(concurrency)
                .try_collect::<()>()
                .await
        }
    };

    let processed = if !matches!(output.format, OutputFormat::Json) {
        let cancel = cancel_on_enter();
        tokio::select! {
            r = process_fut => { r?; true }
            _ = cancel => { false }
        }
    } else {
        process_fut.await?;
        true
    };

    spinner.finish();

    Ok(processed)
}

fn cancel_on_enter() -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    std::thread::spawn(move || {
        let _ = std::io::stdin().read_line(&mut String::new());
        let _ = tx.send(());
    });
    rx
}

/// `topk upload`
pub async fn run(
    client: &Client,
    args: &UploadArgs,
    output: &Output,
) -> Result<UploadResult, Error> {
    let cwd = std::env::current_dir().map_err(Error::IoError)?;

    let files = make_upload_plan(&cwd, &args.patterns, args.recursive)?;
    let total = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    if total == 0 {
        return Ok(UploadResult {
            total: 0,
            uploaded: 0,
            errors: vec![],
            processed: None,
        });
    }

    if args.dry_run {
        return Ok(UploadResult {
            total,
            uploaded: 0,
            errors: vec![],
            processed: None,
        });
    }

    if !output.confirm_or_yes(
        &format!(
            "Upload {} ({}) to '{}' dataset? ",
            format!("{} {}", total, plural(total, "file", "files")).bold(),
            ByteSize(total_size),
            args.dataset
        ),
        args.yes,
    )? {
        return Ok(UploadResult {
            total,
            uploaded: 0,
            errors: vec![],
            processed: None,
        });
    }

    let reporter = upload_reporter(total, output);
    let upload_fut = upload_all(
        client,
        &args.dataset,
        files,
        args.concurrency as usize,
        &*reporter,
    );
    let file_uploads = match tokio::time::timeout(args.timeout, upload_fut).await {
        Ok(out) => out,
        Err(_) => {
            reporter.finish(total, &[]);
            return Err(Error::DeadlineExceeded(format!(
                "upload timed out after {}s",
                args.timeout.as_secs()
            )));
        }
    };

    reporter.finish(total, &file_uploads);

    let uploaded = file_uploads.iter().filter(|f| f.result.is_ok()).count();

    let should_wait = if args.wait {
        true
    } else if !matches!(output.format, OutputFormat::Json) && uploaded > 0 {
        output.confirm(&format!(
            "Wait for processing of {uploaded} uploaded {}? ",
            plural(uploaded, "file", "files")
        ))?
    } else {
        false
    };

    let processed = if should_wait {
        Some(
            wait_for_all(
                client,
                &args.dataset,
                &file_uploads,
                args.concurrency as usize,
                output,
            )
            .await?,
        )
    } else {
        None
    };

    Ok(UploadResult {
        total,
        uploaded,
        errors: file_uploads
            .into_iter()
            .filter_map(|f| {
                f.result.err().map(|e| UploadError {
                    doc_id: f.doc_id,
                    path: Some(f.path),
                    error: e.to_string(),
                })
            })
            .collect(),
        processed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_context::{CliTestContext, OutputJsonExt};
    use assert_cmd::Command;
    use std::fs;
    use tempfile::tempdir;
    use test_context::test_context;
    use uuid::Uuid;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    #[test]
    fn no_files_in_empty_directory() {
        let dir = tempdir().unwrap();
        assert!(make_upload_plan(dir.path(), &["*.md".to_string()], false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn no_files_on_directory_without_recursive() {
        let dir = tempdir().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir(&empty).unwrap();
        assert!(
            make_upload_plan(dir.path(), &[empty.to_string_lossy().into_owned()], false)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn absolute_file_path_is_resolved() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("note.md");
        fs::write(&file, "# note").unwrap();
        let files =
            make_upload_plan(dir.path(), &[file.to_string_lossy().into_owned()], false).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].size > 0);
    }

    #[test]
    fn recursive_globstar_matches_nested_files() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();
        let files = make_upload_plan(dir.path(), &["**/*.md".to_string()], true).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn non_recursive_single_star_matches_only_top_level() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        let files = make_upload_plan(dir.path(), &["*.md".to_string()], false).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn multiple_patterns_are_merged_and_deduplicated() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "# a").unwrap();
        fs::write(dir.path().join("b.pdf"), b"%PDF").unwrap();

        let files = make_upload_plan(
            dir.path(),
            &["a.md".to_string(), "*.md".to_string(), "*.pdf".to_string()],
            false,
        )
        .unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|file| file.path.ends_with("a.md")));
        assert!(files.iter().any(|file| file.path.ends_with("b.pdf")));
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_timeout_aborts(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("timeout");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "-y",
                "--dataset",
                &dataset,
                "--timeout",
                "0",
            ])
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "expected failure due to timeout, got success"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("timed out"), "{stderr}");
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_single_file(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "-y",
                "--dataset",
                &dataset,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 1);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_dry_run(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "markdown.md",
                "--dataset",
                &dataset,
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.uploaded, 0);
    }

    #[test]
    fn make_upload_plan_rejects_unsupported_direct_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("note.txt");
        fs::write(&file, "note").unwrap();

        let err = make_upload_plan(dir.path(), &[file.to_string_lossy().into_owned()], false)
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("Invalid document kind: text/plain"));
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_with_yes_skips_confirmation(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("yes-flag");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", "pdfko.pdf", "-d", &dataset, "-y"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 1);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);

        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();

        let out = cmd()
            .current_dir(dir.path())
            .args(["-o", "json", "upload", "*.md", "-d", &dataset, "--dry-run"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 0);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive_with_globstar_pattern(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);

        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();

        let out = cmd()
            .current_dir(dir.path())
            .args([
                "-o",
                "json",
                "upload",
                "**/*.md",
                "-d",
                &dataset,
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.uploaded, 0);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
    async fn upload_wait(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.client
            .datasets()
            .create(&dataset, Some(ctx.region.clone()))
            .await
            .unwrap();

        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "-d",
                &dataset,
                "-y",
                "--wait",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = out.json().unwrap();
        assert_eq!(result.uploaded, 1);
        assert_eq!(result.processed, Some(true));
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_requires_existing_dataset(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", "pdfko.pdf", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("not found"), "{stderr}");
    }
}
