use std::path::{Path, PathBuf};

use anyhow::Result;
use glob::{glob, PatternError};
use serde::{Deserialize, Serialize};
use topk_rs::{proto::v1::ctx::file::InputFile, Error};

use crate::commands::upload::doc_id_from_path;

use super::mime::MimeType;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFile {
    pub(crate) path: PathBuf,
    pub(crate) doc_id: String,
    pub(crate) size: u64,
    pub(crate) mime_type: MimeType,
}

pub(crate) fn expand_path(pattern: &str) -> Result<PathBuf, Error> {
    let expanded = shellexpand::full(pattern)
        .map_err(|err| Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern, err)))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

pub(crate) fn resolve_files(
    cwd: &Path,
    pattern: &str,
    recursive: bool,
) -> Result<Vec<UploadFile>, Error> {
    let expanded = expand_path(pattern)?;

    let path = if expanded.is_absolute() {
        expanded
    } else {
        cwd.join(expanded)
    };

    let path = path.as_path();

    if path.is_file() {
        return Ok(vec![collect_file(path)?]);
    }

    if path.is_dir() {
        return collect_directory_files(path, recursive);
    }

    collect_files(path)
}

pub(crate) fn collect_file(path: &Path) -> Result<UploadFile, Error> {
    let doc_id = doc_id_from_path(path)?;
    let size = path.metadata().map(|m| m.len())?;
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
    collect_files(&root.join(pattern))
}

pub(crate) fn collect_files(pattern: &Path) -> Result<Vec<UploadFile>, Error> {
    let pattern_str = pattern.to_string_lossy();
    let entries = glob(&pattern_str).map_err(|e: PatternError| {
        Error::InvalidArgument(format!("invalid pattern '{}': {}", pattern_str, e))
    })?;

    // Filter out non-file entries and unsupported MIME types
    entries
        .filter_map(Result::ok)
        .filter(|path| path.is_file())
        .filter(|path| {
            InputFile::guess_mime_type(path)
                .ok()
                .map(|mime| MimeType::from(mime).is_supported())
                .unwrap_or(false)
        })
        .map(|path| collect_file(&path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{collect_directory_files, collect_file, collect_files, expand_path, resolve_files};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn collect_matching_files_filters_by_pattern() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("doc.md"), "# hi").unwrap();
        fs::write(dir.path().join("skip.txt"), "skip").unwrap();
        fs::write(nested.join("report.pdf"), b"%PDF").unwrap();

        let files = collect_files(&dir.path().join("*.md")).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            crate::commands::upload::doc_id_from_path(&dir.path().join("doc.md")).unwrap()
        );
    }

    #[test]
    fn collect_matching_files_matches_single_pattern() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.pdf"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let files = collect_files(&dir.path().join("*.md")).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].doc_id,
            crate::commands::upload::doc_id_from_path(&dir.path().join("a.md")).unwrap()
        );
    }

    #[test]
    fn collect_matching_files_filters_out_unsupported_mime_types() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();
        fs::write(dir.path().join("b.docx"), "").unwrap();
        fs::write(dir.path().join("c.pdf"), "").unwrap();

        let files = collect_files(&dir.path().join("*.*")).unwrap();
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
            crate::commands::upload::doc_id_from_path(&dir.path().join("doc.md")).unwrap()
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

    #[test]
    fn collect_file_sets_size_and_path() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("note.md");
        fs::write(&file, "# note").unwrap();

        let collected = collect_file(&file).unwrap();
        assert_eq!(collected.path, file);
        assert!(collected.size > 0);
    }
}
