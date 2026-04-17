use std::num::NonZeroUsize;

use bytesize::ByteSize;

use topk_rs::{Client, Error};

use crate::output::Output;
use crate::util::plural;

mod plan;
mod progress;
mod result;
mod uploading;
mod waiting;

pub use result::{Totals, UploadError, UploadOutcome, UploadResult};

pub struct UploadArgs<'a> {
    pub client: &'a Client,
    pub dataset: &'a str,
    pub pattern: &'a str,
    pub recursive: bool,
    /// Clamped to a non-zero `usize` internally; clap already enforces `1..=64`.
    pub concurrency: u64,
    pub yes: bool,
    pub dry_run: bool,
    pub wait: bool,
}

/// `topk upload`
pub async fn run(args: UploadArgs<'_>, output: &Output) -> Result<UploadResult, Error> {
    let UploadArgs {
        client,
        dataset,
        pattern,
        recursive,
        concurrency,
        yes,
        dry_run,
        wait,
    } = args;
    let concurrency = NonZeroUsize::new(concurrency as usize)
        .unwrap_or(NonZeroUsize::MIN)
        .get();
    let cwd = std::env::current_dir().map_err(Error::IoError)?;

    let (files, totals) = match plan::build(&cwd, pattern, recursive)? {
        plan::PlanOutcome::NoFiles { message } => {
            return Ok(UploadResult(UploadOutcome::NoFiles { message }));
        }
        plan::PlanOutcome::Files { files, totals } => (files, totals),
    };

    if dry_run {
        return Ok(UploadResult(UploadOutcome::DryRun { totals, files }));
    }

    if !yes
        && !output.confirm(&format!(
            "Upload {} {} ({}) to '{}' dataset? ",
            totals.count,
            plural(totals.count, "file", "files"),
            ByteSize(totals.size),
            dataset
        ))?
    {
        return Ok(UploadResult(UploadOutcome::Skipped { totals }));
    }

    // Phase 1: upload every file concurrently and display a live progress bar.
    let total_count = totals.count;
    let reporter = progress::upload_reporter(total_count, output);
    let uploading::UploadingOutput {
        handles,
        errors: upload_errors,
    } = uploading::upload_all(client, dataset, files, concurrency, &*reporter).await;
    let uploaded = handles.len();
    let failed = upload_errors.len();
    reporter.finish(&progress::summary(total_count, uploaded, failed));
    report_errors(&upload_errors, output);

    // Phase 2: optionally wait for the server to finish processing the handles.
    let should_wait = if wait {
        true
    } else if output.is_human() && uploaded > 0 {
        output.confirm(&format!(
            "Wait for processing of {uploaded} uploaded {}? ",
            plural(uploaded, "file", "files")
        ))?
    } else {
        false
    };

    let processed = if should_wait && !handles.is_empty() {
        Some(waiting::wait_for_all(client, dataset, handles, concurrency, output).await?)
    } else {
        None
    };

    Ok(UploadResult(UploadOutcome::Uploaded {
        totals,
        uploaded,
        errors: upload_errors,
        processed,
    }))
}

fn report_errors(errors: &[UploadError], output: &Output) {
    if !output.is_human() {
        return;
    }
    for e in errors {
        let label = e
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| e.doc_id.clone());
        eprintln!("  {label}: {}", e.error);
    }
}

#[cfg(test)]
mod tests {
    use super::{UploadOutcome, UploadResult};
    use crate::test_context::CliTestContext;
    use crate::util::doc_id_from_path;
    use assert_cmd::Command;
    use std::fs;
    use tempfile::tempdir;
    use test_context::test_context;
    use uuid::Uuid;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_single_file(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "-y",
                "--dataset",
                &dataset,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::Uploaded {
                totals, uploaded, ..
            } => {
                assert_eq!(totals.count, 1);
                assert_eq!(uploaded, 1);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_dry_run(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);
        let path = std::path::Path::new(TESTS_DIR).join("pdfko.pdf");
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "--dataset",
                &dataset,
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::DryRun { totals, files } => {
                assert_eq!(totals.count, 1);
                assert_eq!(files[0].doc_id, doc_id_from_path(&path).unwrap());
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_with_yes_skips_confirmation(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("yes-flag");
        ctx.create_dataset(&dataset);
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", "pdfko.pdf", "-d", &dataset, "-y"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::Uploaded {
                totals, uploaded, ..
            } => {
                assert_eq!(totals.count, 1);
                assert_eq!(uploaded, 1);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);

        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();

        let out = cmd()
            .current_dir(dir.path())
            .args(["-o", "json", "upload", "*.md", "-d", &dataset, "--dry-run"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::DryRun { totals, .. } => {
                assert_eq!(totals.count, 1);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_recursive_with_globstar_pattern(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);

        let dir = tempdir().unwrap();
        let nested = dir.path().join("sub");
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("top.md"), "# top").unwrap();
        fs::write(nested.join("deep.md"), "# deep").unwrap();
        fs::write(nested.join("skip.txt"), "skip").unwrap();

        let out = cmd()
            .current_dir(dir.path())
            .args([
                "-o",
                "json",
                "upload",
                "**/*.md",
                "-d",
                &dataset,
                "--dry-run",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::DryRun { totals, .. } => {
                assert_eq!(totals.count, 2);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
    async fn upload_wait(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.client
            .datasets()
            .create(&dataset, Some(ctx.region.clone()))
            .await
            .unwrap();

        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                "pdfko.pdf",
                "-d",
                &dataset,
                "-y",
                "--wait",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UploadResult = serde_json::from_slice(&out.stdout).unwrap();
        match result.0 {
            UploadOutcome::Uploaded {
                uploaded,
                processed,
                ..
            } => {
                assert_eq!(uploaded, 1);
                assert_eq!(processed, Some(true));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upload_requires_existing_dataset(ctx: &mut CliTestContext) {
        let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", "pdfko.pdf", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("not found"), "{stderr}");
    }
}
