use bytesize::ByteSize;
use regex::Regex;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use topk_rs::{proto::v1::ctx::file::InputFile, Client, Error};
use tracing::info;
use walkdir::WalkDir;

use crate::output::{Output, RenderForHuman};
use crate::util::FileProgress;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Serialize, Deserialize)]
pub struct UploadFile {
    pub(crate) path: PathBuf,
    pub(crate) doc_id: String,
    pub(crate) size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct UploadResult {
    pub(crate) total: usize,
    pub(crate) total_size: u64,
    pub(crate) uploaded: usize,
    pub(crate) processed: bool,
    pub(crate) dry_run: bool,
    pub(crate) skipped: bool,
    pub(crate) files: Vec<UploadFile>,
}

impl RenderForHuman for UploadResult {
    fn render(&self) -> String {
        if self.total == 0 {
            return "No supported files found.".to_string();
        }
        if self.skipped {
            return "Upload skipped.".to_string();
        }
        if self.dry_run {
            let file_word = if self.total == 1 { "file" } else { "files" };
            let mut out = format!(
                "Dry run: would upload {} {} ({}):\n",
                self.total,
                file_word,
                ByteSize(self.total_size)
            );
            for f in &self.files {
                out.push_str(&format!("  {}\n", f.doc_id));
            }
            return out;
        }
        let file_word = if self.uploaded == 1 { "file" } else { "files" };
        if self.processed {
            format!("Uploaded and processed {} {}.", self.uploaded, file_word)
        } else {
            format!("Uploaded {} {}.", self.uploaded, file_word)
        }
    }
}

async fn ensure_dataset(
    client: &Client,
    dataset: &str,
    yes: bool,
    output: &Output,
) -> Result<bool, Error> {
    match client.datasets().get(dataset).await {
        Ok(_) => Ok(false),
        Err(Error::DatasetNotFound) => {
            if yes {
                client.datasets().create(dataset).await?;
                info!(dataset, "created dataset");
                return Ok(true);
            }

            if output.confirm(&format!(
                "Dataset '{}' does not exist. Create it? [y/N] ",
                dataset
            ))? {
                client.datasets().create(dataset).await?;
                info!(dataset, "created dataset");
                return Ok(true);
            }

            Err(Error::InvalidArgument(format!(
                "dataset '{}' does not exist; create it first or pass -y",
                dataset
            )))
        }
        Err(err) => Err(err),
    }
}

/// `topk upload`
pub async fn run(
    client: &Client,
    dataset: &str,
    patterns: &[String],
    concurrency: usize,
    yes: bool,
    dry_run: bool,
    wait: bool,
    no_wait: bool,
    output: &Output,
) -> Result<UploadResult, Error> {
    let regexes = patterns
        .iter()
        .map(|p| {
            Regex::new(p)
                .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", p, e)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cwd = std::env::current_dir().map_err(Error::IoError)?;
    let files = collect_files(&cwd, &regexes)?;
    let total = files.len();

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    info!(files = total, total_size, dataset, concurrency, "uploading");

    if total == 0 {
        return Ok(UploadResult {
            total,
            total_size,
            uploaded: 0,
            processed: false,
            dry_run: false,
            skipped: false,
            files: vec![],
        });
    }

    if dry_run {
        let files: Vec<UploadFile> = files;
        return Ok(UploadResult {
            total,
            total_size,
            uploaded: 0,
            processed: false,
            dry_run: true,
            skipped: false,
            files,
        });
    }

    let dataset_created = ensure_dataset(client, dataset, yes, output).await?;
    if dataset_created {
        output.success(&format!("Dataset '{}' created.", dataset));
    }

    if !yes
        && !output.confirm(&format!(
            "Upload {} files ({}) to dataset '{}'? [y/N] ",
            total,
            ByteSize(total_size),
            dataset
        ))?
    {
        return Ok(UploadResult {
            total,
            total_size,
            uploaded: 0,
            processed: false,
            dry_run: false,
            skipped: true,
            files: vec![],
        });
    }

    // In human mode: wait by default unless --no-wait; in agent mode: only if --wait
    let should_wait = if output.is_human() { !no_wait } else { wait };

    let handle_count = total as u64;
    let done_count = Arc::new(AtomicU64::new(0));

    // Phase 1: upload all files, collect handles
    let upload_progress = FileProgress::new(handle_count);
    let pb_overall = upload_progress.overall.clone();
    let pb_current = upload_progress.current.clone();

    let upload_result = stream::iter(files)
        .map(|file| {
            let client = client.clone();
            let dataset = dataset.to_string();
            let pb_overall = pb_overall.clone();
            let pb_current = pb_current.clone();

            async move {
                if let Some(pb) = &pb_current {
                    pb.set_message(format!("Uploading {}", file.path.display()));
                }
                let input = InputFile::from_path(&file.path)?;
                let handle = client
                    .dataset(&dataset)
                    .upsert_file(
                        file.doc_id.clone(),
                        input,
                        std::iter::empty::<(String, String)>(),
                    )
                    .await?
                    .into_inner()
                    .handle;
                if let Some(pb) = &pb_overall {
                    pb.inc(1);
                }
                Ok::<String, Error>(handle)
            }
        })
        .buffer_unordered(concurrency)
        .try_collect::<Vec<_>>()
        .await;

    upload_progress.finish();
    let handles = upload_result?;
    let uploaded = handles.len();

    if !should_wait || handles.is_empty() {
        return Ok(UploadResult {
            total,
            total_size,
            uploaded,
            processed: false,
            dry_run: false,
            skipped: false,
            files: vec![],
        });
    }

    // Phase 2: wait for processing — cancellable via Enter in interactive mode.
    // Add spinner to a fresh MultiProgress (uploads are done, no conflict).
    let hint = if output.is_human() {
        " — press Enter to skip"
    } else {
        ""
    };
    let processing_multi = if std::io::stderr().is_terminal() {
        Some(indicatif::MultiProgress::new())
    } else {
        None
    };
    let pb_processing: Option<ProgressBar> =
        processing_multi
            .as_ref()
            .map(|multi: &indicatif::MultiProgress| {
                let bar = multi.add(ProgressBar::new_spinner());
                bar.set_style(
                    ProgressStyle::with_template("{spinner:.cyan} {msg}")
                        .expect("valid spinner template"),
                );
                bar.enable_steady_tick(std::time::Duration::from_millis(100));
                bar.set_message(format!("Waiting for processing 0/{handle_count}{hint}"));
                bar
            });

    let process_fut = {
        let client = client.clone();
        let dataset = dataset.to_string();
        let pb_processing = pb_processing.clone();
        let done_count = done_count.clone();

        async move {
            stream::iter(handles)
                .map(|handle| {
                    let client = client.clone();
                    let dataset = dataset.to_string();
                    let pb_processing = pb_processing.clone();
                    let done_count = done_count.clone();

                    async move {
                        let dataset_client = client.dataset(&dataset);
                        dataset_client.wait_for_handle(&handle, None).await?;
                        let done = done_count.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(pb) = &pb_processing {
                            pb.set_message(format!("Processing {done}/{handle_count}{hint}"));
                        }
                        Ok::<_, Error>(())
                    }
                })
                .buffer_unordered(concurrency)
                .try_collect::<()>()
                .await
        }
    };

    // Use a regular OS thread for stdin so the runtime doesn't wait for it on exit.
    // When process_fut wins, the process exits and the OS kills the stdin thread.
    let processed = if output.is_human() {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let _ = std::io::stdin().read_line(&mut String::new());
            let _ = tx.send(());
        });
        tokio::select! {
            r = process_fut => { r?; true }
            _ = rx => { false }
        }
    } else {
        process_fut.await?;
        true
    };

    if let Some(pb) = &pb_processing {
        pb.finish_and_clear();
    }

    Ok(UploadResult {
        total,
        total_size,
        uploaded,
        processed,
        dry_run: false,
        skipped: false,
        files: vec![],
    })
}

pub(crate) fn collect_files(root: &Path, patterns: &[Regex]) -> Result<Vec<UploadFile>, Error> {
    let mut files = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path().to_path_buf();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy();
            if !patterns.iter().any(|re| re.is_match(&rel_str)) {
                return None;
            }
            let size = e.metadata().ok()?.len();
            let doc_id = rel_str.into_owned();
            Some(Ok(UploadFile { doc_id, path, size }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{collect_files, UploadResult};
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use regex::Regex;
    use tempfile::tempdir;
    use test_context::test_context;
    use uuid::Uuid;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_single_file(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                r"pdfko\.pdf",
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 1);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_dry_run(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                r"pdfko\.pdf",
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 0);
        assert!(result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_auto_creates_dataset_with_yes(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("autocreate");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", r"pdfko\.pdf", "-d", &dataset, "-y"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 1);
        assert!(!result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");

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
                r"\.md$",
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        // top.md + sub/deep.md — skip.txt filtered out by pattern
        assert_eq!(result.total, 2);
        assert!(result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
    async fn upload_wait(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.client.datasets().create(&dataset).await.unwrap();

        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                r"pdfko\.pdf",
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.uploaded, 1);
        assert!(result.processed);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_multiple_patterns(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                r"pdfko\.pdf",
                r"markdown\.md",
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.uploaded, 0);
        assert!(result.dry_run);
    }

    #[test]
    fn collect_files_filters_by_pattern() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(dir.path().join("skip.txt"), "skip").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let patterns = vec![Regex::new(r"\.md$").unwrap()];
        let files = collect_files(dir.path(), &patterns).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].doc_id, "doc.md");
    }

    #[test]
    fn collect_files_multiple_patterns_are_unioned() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.pdf"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let patterns = vec![
            Regex::new(r"\.md$").unwrap(),
            Regex::new(r"\.pdf$").unwrap(),
        ];
        let mut files = collect_files(dir.path(), &patterns).unwrap();
        files.sort_by(|a, b| a.doc_id.cmp(&b.doc_id));
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].doc_id, "a.md");
        assert_eq!(files[1].doc_id, "b.pdf");
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_requires_existing_dataset_in_agent_mode_without_yes(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", r"pdfko\.pdf", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("create it first or pass -y"), "{stderr}");
    }
}
