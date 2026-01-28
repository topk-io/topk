use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::proto::ctx::v1::DocumentKind;

#[derive(Clone, Debug)]
pub enum InputSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
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

    pub fn from_bytes(data: &[u8], file_name: String) -> Result<Self, Error> {
        let extension = Path::new(&file_name)
            .extension()
            .ok_or_else(|| {
                Error::Input(anyhow::anyhow!(
                    "Failed to get file extension from file name"
                ))
            })?
            .to_string_lossy()
            .to_string();

        let kind = DocumentKind::from_extension(&extension)?;

        Ok(Self {
            source: InputSource::Bytes(data.to_vec()),
            file_name,
            kind,
        })
    }

    /// Checks if the path is a file
    pub async fn is_file(self) -> Result<Self, Error> {
        match &self.source {
            InputSource::Path(path) => {
                let metadata = tokio::fs::metadata(path).await?;

                if !metadata.is_file() {
                    return Err(Error::Input(anyhow::anyhow!(
                        "Path is not a file: {}",
                        path.display()
                    )));
                }

                Ok(self)
            }
            InputSource::Bytes(_) => Ok(self),
        }
    }
}

impl From<PathBuf> for InputFile {
    fn from(path: PathBuf) -> Self {
        Self::from_path(path).expect("Failed to create InputFile from PathBuf")
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
