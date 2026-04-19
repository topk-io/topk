use std::collections::HashMap;
use std::path::PathBuf;

use futures::stream::{self, StreamExt};
use topk_rs::{
    proto::v1::{ctx::file::InputFile, data::Value},
    Client, Error,
};

use super::progress::ProgressReporter;
use super::result::UploadError;
use crate::util::UploadFile;

struct FileUpload {
    doc_id: String,
    path: PathBuf,
    result: Result<String, Error>,
}

pub struct UploadingOutput {
    pub handles: Vec<String>,
    pub errors: Vec<UploadError>,
}

/// Concurrently upsert every file. The result is split into handles (for the
/// waiting phase) and structured errors, without any partial unwraps.
pub async fn upload_all(
    client: &Client,
    dataset: &str,
    files: Vec<UploadFile>,
    concurrency: usize,
    reporter: &dyn ProgressReporter,
) -> UploadingOutput {
    let outcomes: Vec<FileUpload> = stream::iter(files.into_iter())
        .map(|file| {
            let client = client.clone();
            let dataset = dataset.to_string();

            async move {
                let result = async {
                    let input = InputFile::from_path(&file.path)?;
                    let handle = client
                        .dataset(&dataset)
                        .upsert_file(
                            file.doc_id.clone(),
                            input,
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
        .collect::<Vec<_>>()
        .await;

    let mut handles = Vec::new();
    let mut errors = Vec::new();
    for FileUpload {
        doc_id,
        path,
        result,
    } in outcomes
    {
        match result {
            Ok(handle) => handles.push(handle),
            Err(e) => errors.push(UploadError {
                doc_id,
                path: Some(path),
                error: e.to_string(),
            }),
        }
    }
    UploadingOutput { handles, errors }
}
