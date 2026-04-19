use std::path::Path;

use topk_rs::Error;

use super::result::Totals;
use crate::util::{normalize_glob_pattern, resolve_files, UploadFile};

pub enum PlanOutcome {
    NoFiles {
        message: String,
    },
    Files {
        files: Vec<UploadFile>,
        totals: Totals,
    },
}

pub fn build(cwd: &Path, pattern: &str, recursive: bool) -> Result<PlanOutcome, Error> {
    let match_pattern = normalize_glob_pattern(pattern).to_string();
    let files = resolve_files(cwd, &match_pattern, recursive)?;

    if files.is_empty() {
        let p = Path::new(&match_pattern);
        let target = if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        };
        let recursive_hint = if p.is_dir() && !recursive {
            "; Pass -r or --recursive to match files in sub-directories recursively"
        } else {
            ""
        };
        return Ok(PlanOutcome::NoFiles {
            message: format!(
                "No files found for upload in {}{recursive_hint}.",
                target.display()
            ),
        });
    }

    let totals = Totals {
        count: files.len(),
        size: files.iter().map(|f| f.size).sum(),
    };
    Ok(PlanOutcome::Files { files, totals })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn no_files_in_empty_directory() {
        let dir = tempdir().unwrap();
        let outcome = build(dir.path(), "*.md", false).unwrap();
        match outcome {
            PlanOutcome::NoFiles { message } => {
                assert!(
                    message.contains("No files found"),
                    "unexpected message: {message}"
                );
            }
            _ => panic!("expected NoFiles"),
        }
    }

    #[test]
    fn no_files_on_directory_without_recursive_includes_hint() {
        let dir = tempdir().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir(&empty).unwrap();

        let outcome = build(dir.path(), empty.to_str().unwrap(), false).unwrap();
        match outcome {
            PlanOutcome::NoFiles { message } => {
                assert!(
                    message.contains("--recursive"),
                    "hint missing from: {message}"
                );
            }
            _ => panic!("expected NoFiles"),
        }
    }

    #[test]
    fn absolute_file_path_is_resolved() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("note.md");
        fs::write(&file, "# note").unwrap();

        let outcome = build(dir.path(), file.to_str().unwrap(), false).unwrap();
        match outcome {
            PlanOutcome::Files { files, totals } => {
                assert_eq!(files.len(), 1);
                assert_eq!(totals.count, 1);
                assert!(totals.size > 0);
            }
            _ => panic!("expected Files"),
        }
    }

    #[test]
    fn recursive_globstar_matches_nested_files() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();

        let outcome = build(dir.path(), "**/*.md", true).unwrap();
        match outcome {
            PlanOutcome::Files { totals, .. } => {
                assert_eq!(totals.count, 2);
            }
            _ => panic!("expected Files"),
        }
    }

    #[test]
    fn non_recursive_single_star_matches_only_top_level() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();

        let outcome = build(dir.path(), "*.md", false).unwrap();
        match outcome {
            PlanOutcome::Files { totals, .. } => {
                assert_eq!(totals.count, 1);
            }
            _ => panic!("expected Files"),
        }
    }
}
