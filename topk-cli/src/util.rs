use std::{
    fmt,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use topk_rs::{proto::v1::ctx::file::InputFile, Client, Error};
use walkdir::WalkDir;

use crate::output::Output;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MimeType {
    ApplicationPdf,
    TextMarkdown,
    TextHtml,
    ImagePng,
    ImageJpeg,
    ImageGif,
    ImageWebp,
    ImageTiff,
    ImageBmp,
    Other(String),
}

impl MimeType {
    pub fn is_supported(&self) -> bool {
        !matches!(self, MimeType::Other(_))
    }

    pub fn to_ext(&self) -> &str {
        match self {
            MimeType::ImagePng => "png",
            MimeType::ImageJpeg => "jpg",
            MimeType::ImageGif => "gif",
            MimeType::ImageWebp => "webp",
            MimeType::ImageTiff => "tiff",
            MimeType::ImageBmp => "bmp",
            _ => "bin",
        }
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MimeType::ApplicationPdf => "application/pdf",
            MimeType::TextMarkdown => "text/markdown",
            MimeType::TextHtml => "text/html",
            MimeType::ImagePng => "image/png",
            MimeType::ImageJpeg => "image/jpeg",
            MimeType::ImageGif => "image/gif",
            MimeType::ImageWebp => "image/webp",
            MimeType::ImageTiff => "image/tiff",
            MimeType::ImageBmp => "image/bmp",
            MimeType::Other(s) => s,
        };
        f.write_str(s)
    }
}

impl From<&str> for MimeType {
    fn from(s: &str) -> Self {
        match s {
            "application/pdf" => MimeType::ApplicationPdf,
            "text/markdown" => MimeType::TextMarkdown,
            "text/html" => MimeType::TextHtml,
            "image/png" => MimeType::ImagePng,
            "image/jpeg" => MimeType::ImageJpeg,
            "image/gif" => MimeType::ImageGif,
            "image/webp" => MimeType::ImageWebp,
            "image/tiff" => MimeType::ImageTiff,
            "image/bmp" => MimeType::ImageBmp,
            other => MimeType::Other(other.to_string()),
        }
    }
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        MimeType::from(s.as_str())
    }
}

impl Serialize for MimeType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MimeType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(MimeType::from(s.as_str()))
    }
}

pub fn format_timestamp(rfc3339: &str) -> Option<String> {
    let dt = rfc3339.parse::<DateTime<Utc>>().ok()?;
    Some(dt.with_timezone(&Local).format("%b %-d, %Y %H:%M").to_string())
}

pub(crate) fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprint!("{}", prompt);
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Yes"))
}

pub fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

#[derive(Serialize, Deserialize)]
pub struct UploadFile {
    pub(crate) path: PathBuf,
    pub(crate) doc_id: String,
    pub(crate) size: u64,
    pub(crate) mime_type: MimeType,
}

pub(crate) fn normalize_glob_pattern(pattern: &str) -> &str {
    pattern.strip_prefix("./").unwrap_or(pattern)
}

pub(crate) fn doc_id_from_path(path: &Path) -> Result<String, Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            path.canonicalize()
                .map_err(Error::IoError)?
                .to_string_lossy()
                .as_bytes()
        )
    ))
}

pub(crate) fn resolve_files(cwd: &Path, pattern: &str, recursive: bool) -> Result<Vec<UploadFile>, Error> {
    let path = Path::new(pattern);
    if path.is_file() {
        return Ok(vec![collect_file(path)?]);
    }
    if path.is_dir() {
        return collect_directory_files(path, recursive);
    }
    let top_level_only = !pattern.contains('/');
    let glob = GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, e)))?;
    let globset = GlobSetBuilder::new()
        .add(glob)
        .build()
        .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, e)))?;
    collect_files(cwd, &globset, top_level_only)
}

pub(crate) fn collect_file(path: &Path) -> Result<UploadFile, Error> {
    let doc_id = doc_id_from_path(path)?;
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
    let mime_type = MimeType::from(InputFile::guess_mime_type(path)?);
    Ok(UploadFile {
        doc_id,
        path: path.to_path_buf(),
        size,
        mime_type,
    })
}

pub(crate) fn collect_directory_files(
    root: &Path,
    recursive: bool,
) -> Result<Vec<UploadFile>, Error> {
    let mut walker = WalkDir::new(root).follow_links(false);
    if !recursive {
        walker = walker.max_depth(1);
    }
    walker
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| -> Option<Result<UploadFile, Error>> {
            let mime = MimeType::from(InputFile::guess_mime_type(e.path()).ok()?);
            if !mime.is_supported() {
                return None;
            }
            Some(collect_file(e.path()))
        })
        .collect()
}

pub(crate) fn collect_files(
    root: &Path,
    globset: &GlobSet,
    top_level_only: bool,
) -> Result<Vec<UploadFile>, Error> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| -> Option<Result<UploadFile, Error>> {
            let path = e.path().to_path_buf();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            if !globset.is_match(rel) || (top_level_only && rel.components().count() != 1) {
                return None;
            }
            let mime = MimeType::from(InputFile::guess_mime_type(&path).ok()?);
            if !mime.is_supported() {
                return None;
            }
            Some(collect_file(&path))
        })
        .collect()
}

pub(crate) async fn ensure_dataset(
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

pub struct Spinner {
    pub(crate) bar: Option<ProgressBar>,
    pub(crate) multi: Option<MultiProgress>,
}

impl Spinner {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg}")
    }

    pub fn with_elapsed(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg} [{elapsed}]")
    }

    pub fn disabled() -> Self {
        Self {
            bar: None,
            multi: None,
        }
    }

    fn create(msg: impl Into<String>, template: &str) -> Self {
        if !io::stderr().is_terminal() {
            return Self::disabled();
        }
        let multi = MultiProgress::new();
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(ProgressStyle::with_template(template).expect("valid spinner template"));
        bar.set_message(msg.into());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self {
            bar: Some(bar),
            multi: Some(multi),
        }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        if let Some(pb) = &self.bar {
            pb.set_message(msg.into());
        }
    }

    /// Print a line above the spinner without disrupting it.
    pub fn println(&self, msg: impl AsRef<str>) {
        match &self.multi {
            Some(m) => {
                let _ = m.println(msg.as_ref());
            }
            None => eprintln!("{}", msg.as_ref()),
        }
    }

    pub fn finish(self) {
        if let Some(pb) = self.bar {
            pb.finish_and_clear();
        }
        if let Some(m) = self.multi {
            let _ = m.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_directory_files, collect_files, doc_id_from_path};
    use globset::{Glob, GlobSetBuilder};
    use std::fs;
    use tempfile::tempdir;

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
        let files = collect_files(dir.path(), &globset, true).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            doc_id_from_path(&dir.path().join("doc.md")).unwrap()
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
        let files = collect_files(dir.path(), &globset, true).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            doc_id_from_path(&dir.path().join("a.md")).unwrap()
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
        let files = collect_files(dir.path(), &globset, true).unwrap();
        let paths: Vec<_> = files
            .iter()
            .map(|file| file.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"a.md".to_string()));
        assert!(paths.contains(&"c.pdf".to_string()));
    }

    #[test]
    fn collect_directory_files_non_recursive() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let files = collect_directory_files(dir.path(), false).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            doc_id_from_path(&dir.path().join("doc.md")).unwrap()
        );
    }

    #[test]
    fn collect_directory_files_recursive() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let files = collect_directory_files(dir.path(), true).unwrap();
        assert_eq!(files.len(), 2);
    }
}
