use std::{
    fmt,
    io::{self, IsTerminal, Read},
    path::{Path, PathBuf},
};

use anyhow::Result;

use chrono::{DateTime, Local, Utc};
use globwalk::GlobWalkerBuilder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use topk_rs::{proto::v1::ctx::file::InputFile, Error};

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

pub fn format_timestamp(rfc3339: &str) -> Option<String> {
    let dt = rfc3339.parse::<DateTime<Utc>>().ok()?;
    Some(
        dt.with_timezone(&Local)
            .format("%b %-d, %Y %H:%M")
            .to_string(),
    )
}

pub fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFile {
    pub(crate) path: PathBuf,
    pub(crate) doc_id: String,
    pub(crate) size: u64,
    pub(crate) mime_type: MimeType,
}

pub(crate) fn normalize_glob_pattern(pattern: &str) -> &str {
    pattern.strip_prefix("./").unwrap_or(pattern)
}

pub(crate) fn expand_path(pattern: &str) -> Result<PathBuf, Error> {
    let expanded = shellexpand::full(pattern)
        .map_err(|err| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, err)))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

fn has_glob_chars(component: &str) -> bool {
    component.contains('*')
        || component.contains('?')
        || component.contains('[')
        || component.contains(']')
        || component.contains('{')
        || component.contains('}')
}

fn split_glob_root(cwd: &Path, pattern: &Path) -> (PathBuf, String) {
    let root_base = if pattern.is_absolute() {
        PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
    } else {
        cwd.to_path_buf()
    };

    let mut root = root_base;
    let mut matching_components = Vec::new();
    let mut in_glob = false;

    for component in pattern.components() {
        let part = component.as_os_str().to_string_lossy().to_string();
        if !in_glob && !has_glob_chars(&part) {
            if !matches!(component, std::path::Component::RootDir) {
                root.push(&part);
            }
            continue;
        }
        in_glob = true;
        if !matches!(component, std::path::Component::RootDir) {
            matching_components.push(part);
        }
    }

    let matching_pattern = if matching_components.is_empty() {
        pattern
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        matching_components.join(std::path::MAIN_SEPARATOR_STR)
    };

    (root, matching_pattern)
}

pub(crate) fn resolve_files(
    cwd: &Path,
    pattern: &str,
    recursive: bool,
) -> Result<Vec<UploadFile>, Error> {
    let expanded = expand_path(pattern)?;
    let path = expanded.as_path();
    if path.is_file() {
        return Ok(vec![collect_file(path)?]);
    }
    if path.is_dir() {
        return collect_directory_files(path, recursive);
    }
    let (root, matching_pattern) = split_glob_root(cwd, path);
    collect_matching_files(&root, &matching_pattern)
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
    let pattern = if recursive { "**/*" } else { "*" };
    collect_matching_files(root, pattern)
}

pub(crate) fn collect_matching_files(root: &Path, pattern: &str) -> Result<Vec<UploadFile>, Error> {
    let builder = GlobWalkerBuilder::from_patterns(root, &[pattern]).follow_links(false);
    let builder = if !pattern.contains(std::path::MAIN_SEPARATOR) {
        builder.max_depth(1)
    } else {
        builder
    };

    let walker = builder
        .build()
        .map_err(|e| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, e)))?;

    walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| -> Option<Result<UploadFile, Error>> {
            let path = entry.path();
            let mime = MimeType::from(InputFile::guess_mime_type(path).ok()?);
            if !mime.is_supported() {
                return None;
            }
            Some(collect_file(path))
        })
        .collect()
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

/// Resolves a query string from an optional CLI argument, falling back to stdin.
///
/// Returns:
/// - `Some(query)` if `arg` is provided, or if stdin is piped (non-TTY) and
///   produces non-empty trimmed content.
/// - `None` if `arg` is absent and stdin is either a TTY or empty. Callers
///   typically surface this as "query is required" to the user.
pub fn resolve_query(arg: Option<String>) -> Result<Option<String>> {
    if let Some(q) = arg {
        return Ok(Some(q));
    }

    if io::stdin().is_terminal() {
        return Ok(None);
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;

    let q = buf.trim().to_string();

    if q.is_empty() {
        Ok(None)
    } else {
        Ok(Some(q))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_directory_files, collect_matching_files, doc_id_from_path, expand_path,
        resolve_files, split_glob_root,
    };
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn collect_matching_files_filters_by_pattern() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(dir.path().join("skip.txt"), "skip").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let files = collect_matching_files(dir.path(), "*.md").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            doc_id_from_path(&dir.path().join("doc.md")).unwrap()
        );
    }

    #[test]
    fn collect_matching_files_matches_single_pattern() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.pdf"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let files = collect_matching_files(dir.path(), "*.md").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            doc_id_from_path(&dir.path().join("a.md")).unwrap()
        );
    }

    #[test]
    fn collect_matching_files_filters_out_unsupported_mime_types() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.docx"), "").unwrap();
        fs::write(dir.path().join("c.pdf"), "").unwrap();

        let files = collect_matching_files(dir.path(), "*.*").unwrap();
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

    #[test]
    fn expand_path_resolves_tilde() {
        let home = dirs::home_dir().expect("home dir available");
        assert_eq!(expand_path("~").unwrap(), home);
        assert_eq!(
            expand_path("~/docs/file.pdf").unwrap(),
            home.join("docs/file.pdf")
        );
        assert_eq!(
            expand_path("~/docs/**/*.pdf").unwrap(),
            home.join("docs/**/*.pdf")
        );
    }

    #[test]
    fn split_glob_root_handles_absolute_patterns() {
        let cwd = Path::new("/tmp");
        let pattern = Path::new("/home/jergus/Data/vidore/*/pdfs/*.pdf");
        let (root, matching_pattern) = split_glob_root(cwd, pattern);
        assert_eq!(root, Path::new("/home/jergus/Data/vidore"));
        assert_eq!(matching_pattern, "*/pdfs/*.pdf");
    }

    #[test]
    fn resolve_files_matches_absolute_glob_patterns() {
        let dir = tempdir().unwrap();
        let dataset_dir = dir.path().join("vidore_v3_energy").join("pdfs");
        fs::create_dir_all(&dataset_dir).unwrap();
        let pdf = dataset_dir.join("bilan.pdf");
        fs::write(&pdf, b"%PDF").unwrap();

        let pattern = dir.path().join("*").join("pdfs").join("*.pdf");
        let files = resolve_files(dir.path(), &pattern.to_string_lossy(), false).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, pdf);
    }
}
