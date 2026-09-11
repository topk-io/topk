use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "linux")]
use std::process::Stdio;

use tempfile::TempDir;
#[cfg(target_os = "linux")]
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(target_os = "linux")]
use tokio::time::{timeout, Duration};
use url::Url;

use super::common::tenant_dir;

const AUTH_ISSUER: &str = "https://auth.example.test/";

fn tenant_path(dir: &TempDir) -> PathBuf {
    tenant_dir(&dir.path().join("topk"), &Url::parse(AUTH_ISSUER).unwrap())
}

fn command(dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_topk"));
    for key in [
        "TOPK_API_KEY",
        "TOPK_HOST",
        "TOPK_AUTH_ISSUER",
        "TOPK_AUTH_CLIENT_ID",
        "TOPK_AUTH_AUDIENCE",
        "TOPK_AUTH_CALLBACK_PORTS",
    ] {
        cmd.env_remove(key);
    }
    cmd.env("TOPK_AUTH_ISSUER", AUTH_ISSUER)
        .env("XDG_CONFIG_HOME", dir.path())
        .env("TOPK_AUTH_CALLBACK_PORTS", "0");
    cmd
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
