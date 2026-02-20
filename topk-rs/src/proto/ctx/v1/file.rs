use std::path::PathBuf;

use bytes::Bytes;

use crate::error::Error;

#[derive(Clone, Debug)]
pub enum InputSource {
    Path(PathBuf),
    Bytes(Bytes),
}

#[derive(Clone, Debug)]
pub struct InputFile {
    pub source: InputSource,
    pub file_name: String,
    pub mime_type: String,
}

impl InputFile {
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();

        let file_name = path
            .file_name()
            .ok_or_else(|| Error::Input(anyhow::anyhow!("Failed to get file name")))?
            .to_string_lossy()
            .to_string();

        let mime_type = Self::guess_mime_type(&path)?;

        Ok(Self {
            source: InputSource::Path(path),
            file_name,
            mime_type,
        })
    }

    pub fn from_bytes(
        file_name: impl Into<String>,
        data: impl Into<Bytes>,
        mime_type: impl Into<String>,
    ) -> Result<Self, Error> {
        Ok(Self {
            source: InputSource::Bytes(data.into()),
            file_name: file_name.into(),
            mime_type: mime_type.into(),
        })
    }

    pub fn guess_mime_type(path: impl Into<PathBuf>) -> Result<String, Error> {
        let path = path.into();
        let mime_type = infer::get_from_path(&path)
            .map_err(|e| Error::Input(anyhow::anyhow!(e)))?
            .map(|kind| kind.mime_type().to_string())
            .or_else(|| {
                mime_guess::from_path(&path)
                    .first()
                    .map(|mime| mime.to_string())
            })
            .ok_or_else(|| {
                Error::Input(anyhow::anyhow!(
                    "Could not get MIME type for file: {}",
                    path.display()
                ))
            })?;

        Ok(mime_type)
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;

    use rstest::rstest;

    #[rstest]
    #[case("pdfko.pdf", "application/pdf")]
    #[case("jpeg.jpg", "image/jpeg")]
    #[case("markdown.md", "text/markdown")]
    fn from_path_infers_or_guesses_mime_type(#[case] file: &str, #[case] expected: &str) {
        let input = InputFile::from_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("utils")
                .join("dataset")
                .join(file),
        )
        .expect("failed to create input file from path");
        assert_eq!(input.mime_type, expected);
    }

    #[rstest]
    #[case("markdown")]
    fn from_path_fails_for_no_extension(#[case] file: &str) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("utils")
            .join("dataset")
            .join(file);
        assert!(matches!(
            InputFile::from_path(&path),
            Err(Error::Input(e)) if e.to_string().contains("Could not get MIME type for file")
        ));
    }

    #[test]
    fn from_path_fails_for_nonexistent_file() {
        assert!(matches!(
            InputFile::from_path(&std::env::temp_dir().join("nonexistent_file.pdf")),
            Err(Error::Input(e)) if e.to_string().contains("No such file or directory")
        ));
    }

    #[test]
    fn from_path_fails_for_dir() {
        assert!(matches!(
            InputFile::from_path(&std::env::temp_dir()),
            Err(Error::Input(e)) if e.to_string().contains("Is a directory")
        ));
    }
}
