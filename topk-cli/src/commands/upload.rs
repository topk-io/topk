use bytesize::ByteSize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use topk_rs::{proto::v1::ctx::file::InputFile, Client, Error};
use tracing::info;
use walkdir::WalkDir;

use crate::output::{Output, RenderForHuman};
use crate::util::{FileProgress, Spinner};

const SUPPORTED_EXTENSIONS: &[&str] = &["pdf", "md", "mdx", "jpeg", "jpg", "png"];

struct UploadFile {
    path: PathBuf,
    doc_id: String,
    size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct UploadResult {
    pub total: usize,
    pub total_size: u64,
    pub uploaded: usize,
    pub processed: bool,
    pub dry_run: bool,
    pub skipped: bool,
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
            return format!(
                "Dry run: would upload {} {} ({} total).",
                self.total,
                file_word,
                ByteSize(self.total_size)
            );
        }
        let file_word = if self.uploaded == 1 { "file" } else { "files" };
        if self.processed {
            format!("Uploaded and processed {} {}.", self.uploaded, file_word)
        } else {
            format!("Uploaded {} {}.", self.uploaded, file_word)
        }
    }
}

fn doc_id_for(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    format!("{}/{}", hash, filename)
}

async fn ensure_dataset(
    client: &Client,
    dataset: &str,
    yes: bool,
    dry_run: bool,
    output: &Output,
) -> Result<bool, Error> {
    match client.datasets().get(dataset).await {
        Ok(_) => Ok(false),
        Err(Error::DatasetNotFound) => {
            if dry_run {
                return Err(Error::InvalidArgument(format!(
                    "Dataset '{}' does not exist.",
                    dataset
                )));
            }

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

pub async fn run(
    client: &Client,
    dataset: &str,
    paths: &[PathBuf],
    recursive: bool,
    concurrency: usize,
    yes: bool,
    dry_run: bool,
    wait: bool,
    output: &Output,
) -> Result<UploadResult, Error> {
    let files = collect_files(paths, recursive)?;
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
        });
    }

    let dataset_created = ensure_dataset(client, dataset, yes, dry_run, output).await?;
    if dataset_created {
        output.success(&format!("Dataset '{}' created.", dataset));
    }

    if dry_run {
        return Ok(UploadResult {
            total,
            total_size,
            uploaded: 0,
            processed: false,
            dry_run: true,
            skipped: false,
        });
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
        });
    }

    let progress = FileProgress::new(total as u64);
    let pb_overall = progress.overall.clone();
    let pb_current = progress.current.clone();

    let handles = stream::iter(files)
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

    progress.finish();
    let handles = handles?;
    let uploaded = handles.len();

    if wait {
        let handle_count = handles.len() as u64;
        let done_count = Arc::new(AtomicU64::new(0));

        let spinner = Spinner::new(format!(
            "Waiting for documents to be processed... 0/{}",
            handle_count
        ));
        let pb_waiting = spinner.bar.clone();

        let wait_result = stream::iter(handles)
            .map(|handle| {
                let client = client.clone();
                let dataset = dataset.to_string();
                let pb_waiting = pb_waiting.clone();
                let done_count = done_count.clone();

                async move {
                    client
                        .dataset(&dataset)
                        .wait_for_handle(&handle, None)
                        .await?;
                    if let Some(pb) = &pb_waiting {
                        let done = done_count.fetch_add(1, Ordering::Relaxed) + 1;
                        pb.set_message(format!(
                            "Waiting for documents to be processed... {}/{}",
                            done, handle_count
                        ));
                    }
                    Ok::<(), Error>(())
                }
            })
            .buffer_unordered(concurrency)
            .try_collect::<()>()
            .await;

        spinner.finish();
        wait_result?;
    }

    Ok(UploadResult {
        total,
        total_size,
        uploaded,
        processed: wait,
        dry_run: false,
        skipped: false,
    })
}

fn collect_files(paths: &[PathBuf], recursive: bool) -> Result<Vec<UploadFile>, Error> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            let size = std::fs::metadata(path).map_err(Error::IoError)?.len();
            let path = path.to_path_buf();
            let doc_id = doc_id_for(&path);
            files.push(UploadFile { doc_id, path, size });
            continue;
        }

        if !path.exists() {
            return Err(Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path not found: {}", path.display()),
            )));
        }

        let mut walked = WalkDir::new(path)
            .follow_links(false)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && is_supported(e.path()))
            .map(|e| {
                let path = e.path().to_path_buf();
                let size = e.metadata().map_err(|e| Error::IoError(e.into()))?.len();
                let doc_id = doc_id_for(&path);
                Ok(UploadFile { doc_id, path, size })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        files.append(&mut walked);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);
    Ok(files)
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{run, UploadResult};
    use crate::output::{Output, OutputArg};
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use tempfile::tempdir;
    use test_context::test_context;
    use uuid::Uuid;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_single_file(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", "-y", file, "--dataset", &dataset])
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
        ctx.client.datasets().create(&dataset).await.unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", file, "--dataset", &dataset, "--dry-run"])
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
    async fn upload_dry_run_requires_existing_dataset(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", file, "--dataset", &dataset, "--dry-run"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("Dataset '{}' does not exist.", dataset)),
            "{stderr}"
        );
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_accepts_comma_separated_paths(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.client.datasets().create(&dataset).await.unwrap();

        let pdf = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let md = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let paths = format!("{pdf},{md}");
        let out = cmd()
            .args([
                "--json",
                "upload",
                &paths,
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

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_accepts_explicit_files_and_filters_directory_contents(
        ctx: &mut CliTestContext,
    ) {
        let dataset = ctx.wrap("test");
        ctx.client.datasets().create(&dataset).await.unwrap();

        let dir = tempdir().unwrap();
        let explicit_txt = dir.path().join("test.txt");
        let nested_dir = dir.path().join("nested");
        let supported_md = nested_dir.join("doc.md");
        let unsupported_txt = nested_dir.join("skip.txt");

        fs::create_dir(&nested_dir).unwrap();
        fs::write(&explicit_txt, "hello").unwrap();
        fs::write(&supported_md, "# hi").unwrap();
        fs::write(&unsupported_txt, "skip").unwrap();

        let output = Output::new(true, OutputArg::Agent, false);
        let result = run(
            &ctx.client,
            &dataset,
            &[explicit_txt, nested_dir],
            false,
            4,
            false,
            true,
            false,
            &output,
        )
        .await
        .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.uploaded, 0);
        assert!(result.dry_run);
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_requires_existing_dataset_in_agent_mode_without_yes(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", file, "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("create it first or pass -y"), "{stderr}");
    }
}
