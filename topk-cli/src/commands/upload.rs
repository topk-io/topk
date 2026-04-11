use bytesize::ByteSize;
use globset::{Glob, GlobSetBuilder};
use serde_json::Map;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use topk_rs::{
    proto::v1::{
        ctx::{doc::DocId, file::InputFile},
        data::Value,
    },
    Client, Error,
};
use tracing::info;
use walkdir::WalkDir;

use crate::output::{Output, RenderForHuman};
use crate::util::{
    clear_rendered_block, plural, render_upload_preview, rendered_block_line_count, UploadProgress,
    UploadProgressRow,
};
use indicatif::{ProgressBar, ProgressStyle};

// Mime type allowlist for file upload
const SUPPORTED_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "text/markdown",
    "text/html",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/tiff",
    "image/bmp",
];

#[derive(Serialize, Deserialize)]
pub struct UploadFile {
    pub(crate) path: PathBuf,
    pub(crate) doc_id: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) mime_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct UploadError {
    pub(crate) doc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<PathBuf>,
    pub(crate) error: String,
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
    pub(crate) search_dir: PathBuf,
    pub(crate) pattern: String,
    #[serde(skip)]
    pub(crate) errors_reported: bool,
    #[serde(skip)]
    pub(crate) upload_summary_reported: bool,
}

fn render_upload_errors(errors: &[UploadError]) -> String {
    let mut out = format!("Failed to upload {} file(s):\n", errors.len());
    for e in errors {
        let label = e
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| e.doc_id.clone());
        out.push_str(&format!("{}: {}\n", label, e.error));
    }
    out
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

fn derive_doc_id(path: &Path) -> Result<String, Error> {
    let absolute_path = path.canonicalize().map_err(Error::IoError)?;
    Ok(sha256_hex(absolute_path.to_string_lossy().as_bytes()))
}

fn display_path(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
}

fn sha256_hex(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

fn normalize_glob_pattern(pattern: &str) -> &str {
    pattern.strip_prefix("./").unwrap_or(pattern)
}

fn is_supported_mime_type(mime_type: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime_type)
}

impl RenderForHuman for UploadResult {
    fn render(&self) -> String {
        if self.total_count == 0 {
            let mut out = String::from("No files staged for upload.\n");
            out.push_str(&format!("\nSearched: {}\n", self.search_dir.display()));
            if !self.pattern.is_empty() {
                out.push_str(&format!("Pattern: {}\n", self.pattern));
            }
            return out;
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
        if self.upload_summary_reported && !self.processed {
            if !self.errors.is_empty() && !self.errors_reported {
                return render_upload_errors(&self.errors);
            }
            return String::new();
        }
        let mut out = if self.processed {
            format!(
                "Uploaded and processed {} {}.",
                self.uploaded,
                plural(self.uploaded, "file", "files")
            )
        } else {
            format!(
                "Uploaded {} {}.",
                self.uploaded,
                plural(self.uploaded, "file", "files")
            )
        };
        if !self.errors.is_empty() && !self.errors_reported {
            out.push('\n');
            out.push_str(&render_upload_errors(&self.errors));
        }
        out
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
                return Ok(true);
            }

            if output.confirm(&format!(
                "Dataset '{}' does not exist. Create it? [y/N] ",
                dataset
            ))? {
                client.datasets().create(dataset).await?;
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
    pattern: &str,
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

    // Shell expansion (e.g. zsh `**/*`) produces direct file paths with no metacharacters;
    // those are treated as explicit file references instead of glob patterns.
    let path = Path::new(&match_pattern);
    let has_metachar = match_pattern.contains(['*', '?', '[', '{']);
    let mut files = if !has_metachar && path.is_file() {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let doc_id = derive_doc_id(path)?;
        let mime_type = InputFile::guess_mime_type(path)?.to_string();
        vec![UploadFile {
            doc_id,
            path: path.to_path_buf(),
            size,
            mime_type,
        }]
    } else {
        let glob = Glob::new(&match_pattern)
            .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, e)))?;
        let globset = GlobSetBuilder::new()
            .add(glob)
            .build()
            .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, e)))?;
        collect_files(&cwd, &globset)?
    };

    files.sort_by(|a, b| a.path.cmp(&b.path));

    let total_count = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let upload_rows: Vec<UploadProgressRow> = files
        .iter()
        .map(|file| UploadProgressRow {
            path: display_path(&file.path, &cwd),
            size: file.size,
            mime_type: file.mime_type.clone(),
        })
        .collect();

    info!(total_count, total_size, dataset, concurrency, "uploading");

    if total_count == 0 {
        return Ok(UploadResult {
            total_count,
            total_size,
            search_dir: cwd.clone(),
            pattern: pattern.clone(),
            ..Default::default()
        });
    }

    if id.is_some() && total_count != 1 {
        return Err(Error::InvalidArgument(
            "--id requires exactly one file to upload".to_string(),
        ));
    }

    if !metadata.is_empty() && total_count != 1 {
        return Err(Error::InvalidArgument(
            "--meta requires exactly one file to upload".to_string(),
        ));
    }

    if let Some(id) = id {
        files[0].doc_id = id.into();
    }

    if dry_run {
        let files: Vec<UploadFile> = files;
        return Ok(UploadResult {
            total_count,
            total_size,
            dry_run: true,
            files,
            search_dir: cwd.clone(),
            pattern: pattern.clone(),
            ..Default::default()
        });
    }

    let dataset_created = ensure_dataset(client, dataset, yes, output).await?;

    if dataset_created {
        output.success(&format!("Dataset '{}' created.", dataset));
    }

    let mut preview_lines = 0usize;
    if !yes {
        if output.is_human() {
            let preview = render_upload_preview(
                &format!(
                    "Ready to upload {} {} ({}) to dataset '{}'",
                    total_count,
                    plural(total_count, "file", "files"),
                    ByteSize(total_size),
                    dataset
                ),
                &upload_rows,
            );
            preview_lines = rendered_block_line_count(&preview) + 1;
            eprintln!("{preview}");
        }

        if !output.confirm(&format!(
            "Upload {} files ({}) to dataset '{}'? [y/N] ",
            total_count,
            ByteSize(total_size),
            dataset
        ))? {
            return Ok(UploadResult {
                total_count,
                total_size,
                skipped: true,
                search_dir: cwd.clone(),
                pattern: pattern.clone(),
                ..Default::default()
            });
        }
    }

    if preview_lines > 0 {
        clear_rendered_block(preview_lines);
    }

    let done_count = Arc::new(AtomicU64::new(0));

    // Phase 1: upload all files, collect handles
    let (upload_progress, upload_progress_handle) = UploadProgress::new(upload_rows);

    let results: Vec<(String, PathBuf, Result<String, Error>)> =
        stream::iter(files.into_iter().enumerate())
            .map(|(index, file)| {
                let client = client.clone();
                let dataset = dataset.to_string();
                let metadata = metadata.clone();
                let upload_progress_handle = upload_progress_handle.clone();

                async move {
                    upload_progress_handle.start(index);
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
                    upload_progress_handle.finish(index, result.is_ok());
                    (file.doc_id, file.path, result)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

    let mut handles = Vec::new();
    let mut upload_errors: Vec<UploadError> = Vec::new();
    for (doc_id, path, result) in results {
        match result {
            Ok(handle) => handles.push(handle),
            Err(e) => upload_errors.push(UploadError {
                doc_id,
                path: Some(path),
                error: e.to_string(),
            }),
        }
    }
    let uploaded = handles.len();
    let failed = upload_errors.len();
    let progress_message = if failed == 0 {
        format!(
            "Uploaded {} {}",
            uploaded,
            plural(uploaded, "file", "files")
        )
    } else {
        format!(
            "Uploaded {} {} ({failed} failed)",
            uploaded,
            plural(uploaded, "file", "files")
        )
    };
    upload_progress.finish(progress_message);
    let errors_reported = output.is_human() && !upload_errors.is_empty();
    let upload_summary_reported = output.is_human();
    if errors_reported {
        eprintln!("{}", render_upload_errors(&upload_errors));
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
            processed: false,
            dry_run: false,
            skipped: false,
            files: vec![],
            errors: upload_errors,
            search_dir: cwd,
            pattern,
            errors_reported,
            upload_summary_reported,
        });
    }

    // Phase 2: wait for processing — cancellable via Enter in interactive mode.
    // Add spinner to a fresh MultiProgress (uploads are done, no conflict).
    let processing_count = uploaded as u64;
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
                bar.set_message(format!("Waiting for processing 0/{processing_count}{hint}"));
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
                            pb.set_message(format!("Processing {done}/{processing_count}{hint}"));
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
        total_count,
        total_size,
        uploaded,
        processed,
        dry_run: false,
        skipped: false,
        files: vec![],
        errors: upload_errors,
        search_dir: cwd,
        pattern,
        errors_reported,
        upload_summary_reported,
    })
}

pub(crate) fn collect_files(
    root: &Path,
    globset: &globset::GlobSet,
) -> Result<Vec<UploadFile>, Error> {
    let mut files = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path().to_path_buf();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            if !globset.is_match(rel) {
                return None;
            }
            let mime_type = InputFile::guess_mime_type(&path).ok()?;
            if !is_supported_mime_type(&mime_type) {
                return None;
            }
            let size = e.metadata().ok()?.len();
            let doc_id = derive_doc_id(&path);
            Some(doc_id.map(|doc_id| UploadFile {
                doc_id,
                path,
                size,
                mime_type,
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{collect_files, derive_doc_id, UploadResult};
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use globset::{Glob, GlobSetBuilder};
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
        assert_eq!(result.files[0].doc_id, derive_doc_id(&path).unwrap());
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
        // top.md + sub/deep.md — skip.txt filtered out by pattern
        assert_eq!(result.total_count, 2);
        assert!(result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive_with_dot_slash_pattern(ctx: &mut CliTestContext) {
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
                "./**/*.md",
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

    #[test]
    fn collect_files_filters_by_pattern() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(dir.path().join("skip.txt"), "skip").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let globset = GlobSetBuilder::new()
            .add(Glob::new("*.md").unwrap())
            .build()
            .unwrap();
        let files = collect_files(dir.path(), &globset).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            derive_doc_id(&dir.path().join("doc.md")).unwrap()
        );
    }

    #[test]
    fn collect_files_matches_single_pattern() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.pdf"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let globset = GlobSetBuilder::new()
            .add(Glob::new("*.md").unwrap())
            .build()
            .unwrap();
        let files = collect_files(dir.path(), &globset).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            derive_doc_id(&dir.path().join("a.md")).unwrap()
        );
    }

    #[test]
    fn collect_files_filters_out_unsupported_mime_types() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.docx"), "").unwrap();
        fs::write(dir.path().join("c.pdf"), "").unwrap();

        let globset = GlobSetBuilder::new()
            .add(Glob::new("*.*").unwrap())
            .build()
            .unwrap();
        let files = collect_files(dir.path(), &globset).unwrap();
        let paths: Vec<_> = files
            .iter()
            .map(|file| file.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(paths, vec!["a.md".to_string(), "c.pdf".to_string()]);
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
