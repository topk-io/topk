use bytesize::ByteSize;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Map;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use topk_rs::{
    proto::v1::{
        ctx::{doc::DocId, file::InputFile},
        data::Value,
    },
    Client, Error,
};

use crate::output::{Output, RenderForHuman};
use crate::util::{ensure_dataset, normalize_glob_pattern, plural, resolve_files, UploadFile};

#[derive(Serialize, Deserialize)]
pub struct UploadError {
    pub(crate) doc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<PathBuf>,
    pub(crate) error: String,
}

struct FileUpload {
    doc_id: String,
    path: PathBuf,
    result: Result<String, Error>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UploadResult {
    pub(crate) total_count: usize,
    pub(crate) total_size: u64,
    pub(crate) uploaded: usize,
    pub(crate) processed: bool,
    pub(crate) dry_run: bool,
    pub(crate) skipped: bool,
    pub(crate) files: Vec<UploadFile>,
    pub(crate) errors: Vec<UploadError>,
    #[serde(skip)]
    pub(crate) no_files_message: Option<String>,
}

fn json_to_topk_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::null(),
        serde_json::Value::Bool(b) => Value::bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::i64(i)
            } else {
                Value::f64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone()),
        _ => Value::null(),
    }
}

fn parse_metadata(metadata: Option<String>) -> Result<Map<String, serde_json::Value>, Error> {
    match metadata {
        Some(s) => serde_json::from_str(&s)
            .map_err(|e| Error::InvalidArgument(format!("invalid metadata JSON: {}", e))),
        None => Ok(Map::new()),
    }
}

impl RenderForHuman for UploadResult {
    fn render(&self) -> impl Into<String> {
        if let Some(msg) = &self.no_files_message {
            return msg.clone();
        }

        if self.skipped {
            return "Upload skipped.".to_string();
        }

        if self.dry_run {
            let mut out = format!(
                "Dry run: upload {} {} ({}):\n",
                self.total_count,
                plural(self.total_count, "file", "files"),
                ByteSize(self.total_size)
            );
            for f in &self.files {
                out.push_str(&format!("  {}\n", f.doc_id));
            }
            return out;
        }

        if !self.processed {
            return String::new();
        }

        format!(
            "Uploaded and processed {} {}.",
            self.uploaded,
            plural(self.uploaded, "file", "files")
        )
    }
}

/// `topk upload`
pub async fn run(
    client: &Client,
    dataset: &str,
    pattern: &str,
    recursive: bool,
    id: Option<DocId>,
    metadata: Option<String>,
    concurrency: usize,
    yes: bool,
    dry_run: bool,
    wait: bool,
    output: &Output,
) -> Result<UploadResult, Error> {
    let cwd = std::env::current_dir().map_err(Error::IoError)?;
    let pattern = pattern.to_string();
    let match_pattern = normalize_glob_pattern(&pattern).to_string();
    let metadata = parse_metadata(metadata)?;

    let files = resolve_files(&cwd, &match_pattern, recursive)?;

    let total_count = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    if total_count == 0 {
        let p = Path::new(&match_pattern);
        let target = if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        };
        let recursive_hint = if p.is_dir() && !recursive {
            "; Pass -r or --recursive to match files in sub-directories recursively"
        } else {
            ""
        };
        return Ok(UploadResult {
            no_files_message: Some(format!(
                "No files found for upload in {}{recursive_hint}.",
                target.display()
            )),
            ..Default::default()
        });
    }

    let files = match id {
        Some(_) if total_count != 1 => {
            return Err(Error::InvalidArgument(
                "--id requires exactly one file to upload".to_string(),
            ));
        }
        Some(id) => {
            let mut f = files.into_iter().next().ok_or_else(|| {
                Error::InvalidArgument("--id requires exactly one file to upload".to_string())
            })?;
            f.doc_id = id.into();
            vec![f]
        }
        None => files,
    };

    if dry_run {
        return Ok(UploadResult {
            total_count,
            total_size,
            files,
            dry_run: true,
            ..Default::default()
        });
    }

    let dataset_created = ensure_dataset(client, dataset, yes, output).await?;

    if dataset_created {
        output.success(&format!("Dataset '{}' created.", dataset));
    }

    if !yes {
        if !output.confirm(&format!(
            "Upload {} {} ({}) to '{}' dataset? [y/N] ",
            total_count,
            plural(total_count, "file", "files"),
            ByteSize(total_size),
            dataset
        ))? {
            return Ok(UploadResult {
                total_count,
                total_size,
                skipped: true,
                ..Default::default()
            });
        }
    }

    // Phase 1: upload all files and show progress bar
    let upload_progress = upload_progress_bar(total_count, output);
    let completed_count = Arc::new(AtomicU64::new(0));
    let failed_count = Arc::new(AtomicU64::new(0));

    let outcomes: Vec<FileUpload> = stream::iter(files.into_iter())
        .map(|file| {
            let client = client.clone();
            let dataset = dataset.to_string();
            let metadata = metadata.clone();
            let upload_progress = upload_progress.clone();
            let completed_count = completed_count.clone();
            let failed_count = failed_count.clone();

            async move {
                let result = async {
                    let input = InputFile::from_path(&file.path)?;
                    let metadata = metadata
                        .iter()
                        .map(|(k, v)| (k.clone(), json_to_topk_value(v)));
                    let handle = client
                        .dataset(&dataset)
                        .upsert_file(file.doc_id.clone(), input, metadata)
                        .await?
                        .into_inner()
                        .handle;
                    Ok::<String, Error>(handle)
                }
                .await;

                let completed = completed_count.fetch_add(1, Ordering::Relaxed) + 1;
                let failed = if result.is_err() {
                    failed_count.fetch_add(1, Ordering::Relaxed) + 1
                } else {
                    failed_count.load(Ordering::Relaxed)
                };
                if let Some(pb) = &upload_progress {
                    pb.set_position(completed);
                    let ok = completed.saturating_sub(failed);
                    if failed == 0 {
                        pb.set_message(format!("{ok}/{total_count} uploaded"));
                    } else {
                        pb.set_message(format!("{ok}/{total_count} uploaded, {failed} failed"));
                    }
                }

                FileUpload {
                    doc_id: file.doc_id,
                    path: file.path,
                    result,
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    let (succeeded, failed_outcomes): (Vec<_>, Vec<_>) =
        outcomes.into_iter().partition(|o| o.result.is_ok());
    let handles: Vec<String> = succeeded.into_iter().map(|o| o.result.unwrap()).collect();
    let upload_errors: Vec<UploadError> = failed_outcomes
        .into_iter()
        .map(|o| UploadError {
            doc_id: o.doc_id,
            path: Some(o.path),
            error: o.result.unwrap_err().to_string(),
        })
        .collect();
    let uploaded = handles.len();
    let failed = upload_errors.len();
    let progress_message = if failed == 0 {
        format!(
            "{uploaded}/{total_count} {} uploaded",
            plural(total_count, "file", "files")
        )
    } else {
        format!(
            "{uploaded}/{total_count} {} uploaded ({failed} failed)",
            plural(total_count, "file", "files")
        )
    };
    if let Some(pb) = upload_progress {
        pb.finish_with_message(progress_message);
        eprintln!();
    }
    if output.is_human() {
        for e in &upload_errors {
            let label = e
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| e.doc_id.clone());
            eprintln!("  {label}: {}", e.error);
        }
    }

    let should_wait = if wait {
        true
    } else if output.is_human() && uploaded > 0 {
        output.confirm(&format!(
            "Wait for processing of {uploaded} uploaded {}? [y/N] ",
            plural(uploaded, "file", "files")
        ))?
    } else {
        false
    };

    if !should_wait || handles.is_empty() {
        return Ok(UploadResult {
            total_count,
            total_size,
            uploaded,
            errors: upload_errors,
            ..Default::default()
        });
    }

    // Phase 2: wait for processing — cancellable via Enter in interactive mode.
    let processing_count = uploaded as u64;
    let spinner = output.spinner(format!(
        "0/{processing_count} processed — press Enter to skip"
    ));
    let pb = spinner.bar.clone();

    let process_fut = {
        let client = client.clone();
        let dataset = dataset.to_string();

        async move {
            let done_count = Arc::new(AtomicU64::new(0));
            stream::iter(handles)
                .map(|handle| {
                    let client = client.clone();
                    let dataset = dataset.to_string();
                    let pb = pb.clone();
                    let done_count = done_count.clone();

                    async move {
                        client
                            .dataset(&dataset)
                            .wait_for_handle(&handle, None)
                            .await?;
                        let done = done_count.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(pb) = &pb {
                            pb.set_message(format!(
                                "{done}/{processing_count} processed — press Enter to skip"
                            ));
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

    spinner.finish();

    Ok(UploadResult {
        total_count,
        total_size,
        uploaded,
        processed,
        errors: upload_errors,
        ..Default::default()
    })
}

fn upload_progress_bar(total_count: usize, output: &Output) -> Option<ProgressBar> {
    if !output.is_human() || !std::io::stderr().is_terminal() {
        return None;
    }

    let pb = ProgressBar::new(total_count as u64);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg}")
            .expect("valid upload progress template")
            .progress_chars("=>-"),
    );
    pb.set_message(format!("0/{total_count} uploaded"));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    Some(pb)
}

#[cfg(test)]
mod tests {
    use super::UploadResult;
    use crate::test_context::CliTestContext;
    use crate::util::doc_id_from_path;
    use assert_cmd::Command;
    use std::fs;
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total_count, 1);
        assert_eq!(result.uploaded, 1);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_dry_run(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        let path = std::path::Path::new(TESTS_DIR).join("pdfko.pdf");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
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
        assert_eq!(result.total_count, 1);
        assert_eq!(result.uploaded, 0);
        assert!(result.dry_run);
        assert_eq!(result.files[0].doc_id, doc_id_from_path(&path).unwrap());
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_auto_creates_dataset_with_yes(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("autocreate");
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total_count, 1);
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
            .args(["-o", "json", "upload", "*.md", "-d", &dataset, "--dry-run"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total_count, 1);
        assert!(result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive_with_globstar_pattern(ctx: &mut CliTestContext) {
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total_count, 2);
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
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.uploaded, 1);
        assert!(result.processed);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_single_file_accepts_custom_id(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "--dataset",
                &dataset,
                "--id",
                "custom-doc-id",
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
        assert_eq!(result.files[0].doc_id, "custom-doc-id");
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_rejects_custom_id_for_pattern_matching_multiple_files(
        ctx: &mut CliTestContext,
    ) {
        let dataset = ctx.wrap("test");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "*.*",
                "--dataset",
                &dataset,
                "--id",
                "custom-doc-id",
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--id requires exactly one file"),
            "{stderr}"
        );
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_requires_existing_dataset_in_agent_mode_without_yes(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", "pdfko.pdf", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("create it first or pass -y"), "{stderr}");
    }
}
