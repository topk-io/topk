use std::path::PathBuf;

use crate::error::Error;
use crate::proto::ctx::v1::DocumentKind;

#[derive(Clone, Debug)]
pub struct InputFile {
    // Path to the file
    pub path: PathBuf,
    // File name
    pub file_name: String,
    // Document kind
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
            path,
            file_name,
            kind,
        })
    }
}

#[derive(Clone)]
pub struct FileId(String);

impl From<FileId> for String {
    fn from(file_id: FileId) -> Self {
        file_id.0
    }
}

impl From<&str> for FileId {
    fn from(s: &str) -> Self {
        FileId(s.to_string())
    }
}

impl From<String> for FileId {
    fn from(s: String) -> Self {
        FileId(s)
    }
}
