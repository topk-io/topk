use std::path::PathBuf;

use bytes::Bytes;

use crate::error::Error;
use crate::proto::ctx::v1::DocumentKind;

#[derive(Clone, Debug)]
pub enum InputSource {
    Path(PathBuf),
    Bytes(Bytes),
}

#[derive(Clone, Debug)]
pub struct InputFile {
    pub source: InputSource,
    pub file_name: String,
    pub kind: DocumentKind,
}

impl InputFile {
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();

        let file_name = path
            .file_name()
            .ok_or_else(|| Error::Input(anyhow::anyhow!("Failed to get file name")))?
            .to_string_lossy()
            .to_string();

        let extension = path
            .extension()
            .ok_or_else(|| Error::Input(anyhow::anyhow!("Failed to get file extension")))?
            .to_string_lossy()
            .to_string();

        let kind = DocumentKind::from_extension(&extension)?;

        Ok(Self {
            source: InputSource::Path(path),
            file_name,
            kind,
        })
    }

    pub fn from_bytes(
        data: impl Into<Bytes>,
        file_name: String,
        kind: DocumentKind,
    ) -> Result<Self, Error> {
        Ok(Self {
            source: InputSource::Bytes(data.into()),
            file_name,
            kind,
        })
    }

    /// Returns `true` if the provided [`InputFile`] is a file path.
    pub async fn is_file(&self) -> Result<bool, Error> {
        match &self.source {
            InputSource::Path(path) => {
                let metadata = tokio::fs::metadata(path).await?;
                Ok(metadata.is_file())
            }
            InputSource::Bytes(_) => Ok(true),
        }
    }
}

#[derive(Clone)]
pub struct FileId(String);

impl From<FileId> for String {
    fn from(file_id: FileId) -> Self {
        file_id.0
    }
}

impl From<String> for FileId {
    fn from(s: String) -> Self {
        FileId(s)
    }
}

impl From<&str> for FileId {
    fn from(s: &str) -> Self {
        FileId(s.to_string())
    }
}
