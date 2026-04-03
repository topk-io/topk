use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use tracing::info;
use walkdir::WalkDir;
use topk_rs::{proto::v1::ctx::file::InputFile, Client, Error};

use crate::output::RenderForHuman;
use crate::util::{FileProgress, Spinner};

const SUPPORTED_EXTENSIONS: &[&str] = &["pdf", "md", "mdx", "jpeg", "jpg", "png"];

struct UploadFile {
    path: PathBuf,
    doc_id: String,
    size: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadError {
    pub path: String,
    pub error: String,
}

#[derive(Serialize, serde::Deserialize)]
pub struct UploadResult {
    pub total: usize,
    pub uploaded: usize,
    pub processed: bool,
    pub dry_run: bool,
}

impl RenderForHuman for UploadResult {
    fn render(&self) -> String {
        if self.total == 0 {
            return "No supported files found.".to_string();
        }
        if self.dry_run {
            return String::new();
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
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    format!("{}/{}", hash, filename)
}

async fn ensure_dataset(client: &Client, dataset: &str) -> Result<(), Error> {
    match client.datasets().get(dataset).await {
        Ok(_) => Ok(()),
        Err(_) => {
            client.datasets().create(dataset).await?;
            info!(dataset, "created dataset");
            Ok(())
        }
    }
}

pub async fn run(
    client: &Client,
    dataset: &str,
    path: &Path,
    recursive: bool,
    concurrency: usize,
    dry_run: bool,
    wait: bool,
) -> Result<(UploadResult, Vec<UploadError>), Error> {
    let files = collect_files(path, recursive)?;
    let total = files.len();

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    info!(files = total, total_size, dataset, concurrency, "uploading");

    if dry_run {
        return Ok((UploadResult { total, uploaded: 0, processed: false, dry_run: true }, vec![]));
    }

    ensure_dataset(client, dataset).await?;

    let progress = FileProgress::new(total as u64);
    let pb_overall = progress.overall.clone();
    let pb_current = progress.current.clone();

    let errors: Arc<Mutex<Vec<UploadError>>> = Arc::new(Mutex::new(Vec::new()));
    let upload_errors = Arc::new(AtomicU64::new(0));

    let handles: Vec<String> = stream::iter(files)
        .map(|file| {
            let client = client.clone();
            let dataset = dataset.to_string();
            let pb_overall = pb_overall.clone();
            let pb_current = pb_current.clone();
            let errors = errors.clone();
            let upload_errors = upload_errors.clone();

            async move {
                if let Some(pb) = &pb_current {
                    pb.set_message(format!("Uploading {}", file.path.display()));
                }

                let input = match InputFile::from_path(&file.path) {
                    Ok(i) => i,
                    Err(e) => {
                        upload_errors.fetch_add(1, Ordering::Relaxed);
                        errors.lock().unwrap().push(UploadError {
                            path: file.path.display().to_string(),
                            error: e.to_string(),
                        });
                        if let Some(pb) = &pb_overall { pb.inc(1); }
                        return None;
                    }
                };

                let handle = match client
                    .dataset(&dataset)
                    .upsert_file(file.doc_id.clone(), input, std::iter::empty::<(String, String)>())
                    .await
                {
                    Ok(resp) => resp.into_inner().handle,
                    Err(e) => {
                        upload_errors.fetch_add(1, Ordering::Relaxed);
                        errors.lock().unwrap().push(UploadError {
                            path: file.path.display().to_string(),
                            error: e.to_string(),
                        });
                        if let Some(pb) = &pb_overall { pb.inc(1); }
                        return None;
                    }
                };

                if let Some(pb) = &pb_overall { pb.inc(1); }
                Some(handle)
            }
        })
        .buffer_unordered(concurrency)
        .filter_map(|h| async move { h })
        .collect()
        .await;

    progress.finish();

    if wait {
        let handle_count = handles.len() as u64;
        let done_count = Arc::new(AtomicU64::new(0));

        let spinner = Spinner::new(format!("Waiting for documents to be processed... 0/{}", handle_count));
        let pb_waiting = spinner.bar.clone();

        stream::iter(handles)
            .map(|handle| {
                let client = client.clone();
                let dataset = dataset.to_string();
                let pb_waiting = pb_waiting.clone();
                let errors = errors.clone();
                let done_count = done_count.clone();

                async move {
                    if let Err(e) = client.dataset(&dataset).wait_for_handle(&handle, None).await {
                        errors.lock().unwrap().push(UploadError {
                            path: String::new(),
                            error: e.to_string(),
                        });
                    }
                    if let Some(pb) = &pb_waiting {
                        let done = done_count.fetch_add(1, Ordering::Relaxed) + 1;
                        pb.set_message(format!(
                            "Waiting for documents to be processed... {}/{}",
                            done, handle_count
                        ));
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<()>()
            .await;

        spinner.finish();
    }

    let errors = Arc::try_unwrap(errors)
        .map_err(|_| Error::Internal("upload tasks still running".to_string()))?
        .into_inner()
        .map_err(|_| Error::Internal("mutex poisoned".to_string()))?;
    let uploaded = total.saturating_sub(upload_errors.load(Ordering::Relaxed) as usize);

    Ok((UploadResult { total, uploaded, processed: wait, dry_run: false }, errors))
}

fn collect_files(path: &Path, recursive: bool) -> Result<Vec<UploadFile>, Error> {
    let mut files: Vec<UploadFile> = WalkDir::new(path)
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
        .collect::<Result<_, Error>>()?;

    files.sort_by(|a, b| a.path.cmp(&b.path));
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
    use assert_cmd::Command;
    use uuid::Uuid;
    use super::UploadResult;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    fn unique_name() -> String {
        format!("topk-cli-{}", Uuid::new_v4().simple())
    }

    fn delete_dataset(name: &str) {
        cmd().args(["dataset", "delete", "--dataset", name, "-y"]).output().unwrap();
    }

    #[test]
    fn upload_single_file() {
        let dataset = unique_name();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", file, "--dataset", &dataset])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 1);

        delete_dataset(&dataset);
    }

    #[test]
    fn upload_dry_run() {
        let dataset = unique_name();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upload", file, "--dataset", &dataset, "--dry-run"])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.uploaded, 0);
        assert!(result.dry_run);
    }
}
