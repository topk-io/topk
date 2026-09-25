use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::{Args, Command as ClapCommand, FromArgMatches};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{timeout, Duration};
use url::Url;

use topk::endpoint::DataEndpoint;

use super::common::tenant_dir;

const AUTH_ISSUER: &str = "https://auth.example.test/";

fn tenant_path(dir: &TempDir) -> PathBuf {
    tenant_dir(&dir.path().join("topk"), &Url::parse(AUTH_ISSUER).unwrap())
}

fn command(dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_topk"));
    for key in [
        "TOPK_API_KEY",
        "TOPK_CONFIG_DIR",
        "TOPK_REGION",
        "TOPK_HOST",
        "TOPK_AUTH_ISSUER",
        "TOPK_AUTH_CLIENT_ID",
        "TOPK_AUTH_AUDIENCE",
        "TOPK_AUTH_CALLBACK_PORTS",
    ] {
        cmd.env_remove(key);
    }
    cmd.env("TOPK_AUTH_ISSUER", AUTH_ISSUER)
        .env("TOPK_CONFIG_DIR", dir.path().join("topk"))
        .env("TOPK_AUTH_CALLBACK_PORTS", "0");
    cmd
}

#[test]
fn logout_of_missing_session_is_idempotent_across_configurations() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("topk")).unwrap();
    for audience in [
        "https://api.one.test",
        "https://api.two.test",
        "https://api.one.test",
    ] {
        assert!(command(&dir)
            .args(["logout", "--auth-audience", audience])
            .output()
            .unwrap()
            .status
            .success());
    }
    assert!(!tenant_path(&dir).join("credentials.toml").exists());
    assert!(!dir.path().join("topk/config.toml").exists());
    assert!(tenant_path(&dir).join("session.lock").exists());
}

#[tokio::test]
async fn no_browser_prints_login_url_without_saving_credentials() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("topk")).unwrap();
    let mut child = tokio::process::Command::from(command(&dir))
        .kill_on_drop(true)
        .args(["login", "--no-browser"])
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    timeout(Duration::from_secs(10), stderr.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    assert!(line.contains("Open this URL"), "{line}");
    assert!(!dir.path().join("topk/config.toml").exists());
    assert!(!tenant_path(&dir).join("credentials.toml").exists());
}

#[test]
fn invalid_authentication_configuration_is_rejected_during_parsing() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("topk")).unwrap();
    for issuer in ["", "example.com", "https://", "https://invalid host/"] {
        for from_env in [false, true] {
            let mut cmd = command(&dir);
            cmd.arg("logout");
            if from_env {
                cmd.env("TOPK_AUTH_ISSUER", issuer);
            } else {
                cmd.args(["--auth-issuer", issuer]);
            }
            let output = cmd.output().unwrap();
            assert_eq!(output.status.code(), Some(2), "issuer: {issuer:?}");
            assert!(String::from_utf8(output.stderr)
                .unwrap()
                .contains("--auth-issuer"));
        }
    }
    let output = command(&dir)
        .args(["logout", "--auth-client-id", ""])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!tenant_path(&dir).join("credentials.toml").exists());
}

#[test]
fn logout_without_a_session_cleans_up_legacy_api_key() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("topk")).unwrap();
    let path = dir.path().join("topk/config.toml");
    std::fs::write(
        &path,
        "api_key = 'legacy-secret'\n[preferences]\ncolor = false\n",
    )
    .unwrap();
    for _ in 0..2 {
        let output = command(&dir).arg("logout").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(!path.exists());
    assert!(tenant_path(&dir).join("session.lock").exists());
}

#[test]
fn config_directory_override_is_not_a_cli_option() {
    let dir = TempDir::new().unwrap();
    let output = command(&dir)
        .args(["logout", "--config-dir"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("unexpected argument '--config-dir'"));
}
#[cfg(feature = "import")]
#[test]
fn import_accepts_project_option() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("books.jsonl");
    std::fs::write(&source, "{\"_id\":\"one\",\"title\":\"Book\"}\n").unwrap();
    let output = command(&dir)
        .arg("import")
        .arg(&source)
        .args(["--dry-run", "--to", "books", "--project-id", "p1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(feature = "import")]
#[test]
fn local_dry_run_requires_no_authentication() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("books.jsonl");
    std::fs::write(&source, "{\"_id\":\"one\",\"title\":\"Book\"}\n").unwrap();
    let output = command(&dir)
        .arg("import")
        .arg(&source)
        .args(["--to", "books", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (args, message) in [
        (vec![], "--region is required"),
        (vec!["--region", "test"], "--project-id is required"),
    ] {
        let output = command(&dir)
            .arg("import")
            .arg(&source)
            .args(["--to", "books", "--yes"])
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
async fn endpoint_selects_api_key_or_project_authentication() {
    let parse = |args: &[&str]| {
        let matches = DataEndpoint::augment_args(ClapCommand::new("test"))
            .mut_args(|arg| arg.env(None::<&str>))
            .get_matches_from(args);
        DataEndpoint::from_arg_matches(&matches).unwrap()
    };
    let endpoint = parse(&[
        "test",
        "--region",
        "test",
        "--api-key",
        "key",
        "--project-id",
        "p1",
    ]);
    assert!(endpoint
        .client()
        .err()
        .unwrap()
        .to_string()
        .contains("--project-id cannot be combined with an API key"));
    let client = parse(&["test", "--region", "test", "--api-key", "key"])
        .client()
        .unwrap();
    assert_eq!(client.config().headers()["authorization"], "Bearer key");
    let endpoint = parse(&["test", "--region", "test", "--project-id", "p1"]);
    let client = endpoint.client().unwrap();
    assert!(!client.config().headers().contains_key("authorization"));
    assert_eq!(client.config().region(), Some("test"));
    assert!(parse(&["test", "--region", "test"])
        .client()
        .err()
        .unwrap()
        .to_string()
        .contains("--project-id is required"));
}

#[test]
fn logout_clears_project_tokens() {
    let dir = TempDir::new().unwrap();
    let tokens = tenant_path(&dir).join("projects").join("tokens");
    std::fs::create_dir_all(&tokens).unwrap();
    std::fs::write(
        tenant_path(&dir).join("credentials.toml"),
        "account credentials",
    )
    .unwrap();
    std::fs::write(tokens.join("p1.toml"), "project token").unwrap();
    let result = command(&dir).arg("logout").output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!tenant_path(&dir).join("credentials.toml").exists());
    assert!(!tokens.exists());
}
