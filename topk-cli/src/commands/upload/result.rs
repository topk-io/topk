use std::path::PathBuf;

use bytesize::ByteSize;
use serde::{Deserialize, Serialize};

use crate::output::RenderForHuman;
use crate::util::{plural, UploadFile};

#[derive(Debug, Serialize, Deserialize)]
pub struct Totals {
    pub count: usize,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadError {
    pub doc_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UploadOutcome {
    NoFiles {
        message: String,
    },
    DryRun {
        totals: Totals,
        files: Vec<UploadFile>,
    },
    Skipped {
        totals: Totals,
    },
    Uploaded {
        totals: Totals,
        uploaded: usize,
        errors: Vec<UploadError>,
        #[serde(skip_serializing_if = "Option::is_none")]
        processed: Option<bool>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UploadResult(pub UploadOutcome);

impl RenderForHuman for UploadResult {
    fn render(&self) -> impl Into<String> {
        match &self.0 {
            UploadOutcome::NoFiles { message } => message.clone(),
            UploadOutcome::Skipped { .. } => "Upload skipped.".to_string(),
            UploadOutcome::DryRun { totals, files } => {
                let mut out = format!(
                    "Dry run: upload {} {} ({}):\n",
                    totals.count,
                    plural(totals.count, "file", "files"),
                    ByteSize(totals.size)
                );
                for f in files {
                    out.push_str(&format!("  {}\n", f.doc_id));
                }
                out
            }
            UploadOutcome::Uploaded {
                uploaded,
                processed,
                ..
            } => match processed {
                Some(true) => format!(
                    "Uploaded and processed {} {}.",
                    uploaded,
                    plural(*uploaded, "file", "files")
                ),
                // Waited, but the user pressed Enter to skip the wait.
                Some(false) => format!(
                    "Uploaded {} {}; processing skipped.",
                    uploaded,
                    plural(*uploaded, "file", "files")
                ),
                // User did not opt in to waiting — stay silent (matches old behavior).
                None => String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals() -> Totals {
        Totals {
            count: 2,
            size: 1024,
        }
    }

    #[test]
    fn no_files_roundtrip() {
        let outcome = UploadOutcome::NoFiles {
            message: "No files found for upload in ./docs.".to_string(),
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["kind"], "no_files");
        assert_eq!(json["message"], "No files found for upload in ./docs.");
        let parsed: UploadOutcome = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed, UploadOutcome::NoFiles { .. }));
    }

    #[test]
    fn dry_run_roundtrip() {
        let outcome = UploadOutcome::DryRun {
            totals: totals(),
            files: vec![],
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["kind"], "dry_run");
        assert_eq!(json["totals"]["count"], 2);
        assert_eq!(json["totals"]["size"], 1024);
        let parsed: UploadOutcome = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed, UploadOutcome::DryRun { .. }));
    }

    #[test]
    fn skipped_roundtrip() {
        let outcome = UploadOutcome::Skipped { totals: totals() };
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["kind"], "skipped");
        let parsed: UploadOutcome = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed, UploadOutcome::Skipped { .. }));
    }

    #[test]
    fn uploaded_roundtrip() {
        let outcome = UploadOutcome::Uploaded {
            totals: totals(),
            uploaded: 2,
            errors: vec![],
            processed: Some(true),
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["kind"], "uploaded");
        assert_eq!(json["uploaded"], 2);
        assert_eq!(json["processed"], true);
        let parsed: UploadOutcome = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed, UploadOutcome::Uploaded { .. }));
    }

    #[test]
    fn uploaded_omits_processed_when_none() {
        let outcome = UploadOutcome::Uploaded {
            totals: totals(),
            uploaded: 2,
            errors: vec![],
            processed: None,
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert!(json.get("processed").is_none());
    }

    #[test]
    fn upload_result_is_transparent() {
        let result = UploadResult(UploadOutcome::Skipped { totals: totals() });
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["kind"], "skipped");
    }

    #[test]
    fn render_no_files() {
        let r = UploadResult(UploadOutcome::NoFiles {
            message: "nothing here".to_string(),
        });
        assert_eq!(r.render().into(), "nothing here".to_string());
    }

    #[test]
    fn render_skipped() {
        let r = UploadResult(UploadOutcome::Skipped { totals: totals() });
        assert_eq!(r.render().into(), "Upload skipped.".to_string());
    }

    #[test]
    fn render_uploaded_processed() {
        let r = UploadResult(UploadOutcome::Uploaded {
            totals: totals(),
            uploaded: 1,
            errors: vec![],
            processed: Some(true),
        });
        assert_eq!(
            r.render().into(),
            "Uploaded and processed 1 file.".to_string()
        );
    }

    #[test]
    fn render_uploaded_without_wait_is_silent() {
        let r = UploadResult(UploadOutcome::Uploaded {
            totals: totals(),
            uploaded: 3,
            errors: vec![],
            processed: None,
        });
        assert_eq!(r.render().into(), String::new());
    }
}
