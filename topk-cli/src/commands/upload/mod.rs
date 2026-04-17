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

/// `topk upload`
#[allow(clippy::too_many_arguments)]
pub async fn run(
    client: &Client,
    dataset: &str,
    pattern: &str,
    recursive: bool,
    concurrency: usize,
    yes: bool,
    dry_run: bool,
    wait: bool,
    output: &Output,
) -> Result<UploadResult, Error> {
    let cwd = std::env::current_dir().map_err(Error::IoError)?;

    // Collect files to upload and compute totals.
    let (files, totals) = match plan::build(&cwd, pattern, recursive)? {
        plan::PlanOutcome::NoFiles { message } => {
            return Ok(UploadResult(UploadOutcome::NoFiles { message }));
        }
        plan::PlanOutcome::Files { files, totals } => (files, totals),
    };

    // If dry run, return the result immediately.
    if dry_run {
        return Ok(UploadResult(UploadOutcome::DryRun { totals, files }));
    }

    // Skip upload if the user did not confirm or --yes was not passed.
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

    // Upload every file concurrently and display a live progress bar.
    let reporter = progress::upload_reporter(totals.count, output);
    let uploading_output =
        uploading::upload_all(client, dataset, files, concurrency, &*reporter).await;
    let uploaded = uploading_output.handles.len();
    let failed = uploading_output.errors.len();

    // Finish the progress bar and report errors if any.
    reporter.finish(
        &progress::summary(totals.count, uploaded, failed),
        &uploading_output.errors,
    );

    // (Optional) Wait for the server to finish processing the handles.
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

    let processed = if should_wait && !uploading_output.handles.is_empty() {
        Some(
            waiting::wait_for_all(
                client,
                dataset,
                uploading_output.handles,
                concurrency,
                output,
            )
            .await?,
        )
    } else {
        None
    };

    Ok(UploadResult(UploadOutcome::Uploaded {
        totals,
        uploaded,
        errors: uploading_output.errors,
        processed,
    }))
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
