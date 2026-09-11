use std::process::Command;
#[cfg(target_os = "linux")]
use std::process::Stdio;

use tempfile::TempDir;
#[cfg(target_os = "linux")]
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(target_os = "linux")]
use tokio::time::{timeout, Duration};

fn command(dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_topk"));
    for key in [
        "TOPK_API_KEY",
        "TOPK_HOST",
        "TOPK_AUTH_DOMAIN",
        "TOPK_AUTH_CLIENT_ID",
        "TOPK_AUTH_AUDIENCE",
    ] {
        cmd.env_remove(key);
    }
    cmd.env("XDG_CONFIG_HOME", dir.path());
    cmd
}

#[cfg(target_os = "linux")]
#[test]
fn login_checks_storage_before_printing_authorization_url() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("topk")).unwrap();
    std::fs::create_dir(dir.path().join("topk/credentials.toml")).unwrap();
    let output = command(&dir)
        .args(["login", "--no-browser"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error:"));
    assert!(!stderr.contains("/authorize"));
}

#[cfg(target_os = "linux")]
#[test]
fn logout_of_missing_session_is_idempotent_across_configurations() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("topk")).unwrap();
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
    assert!(!dir.path().join("topk/credentials.toml").exists());
    assert!(!dir.path().join("topk/config.toml").exists());
    assert!(dir.path().join("topk/session.lock").exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn no_browser_prints_login_url_without_saving_credentials() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("topk")).unwrap();
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
    assert!(!dir.path().join("topk/credentials.toml").exists());
}

#[test]
fn invalid_authentication_configuration_is_rejected_during_parsing() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("topk")).unwrap();
    for domain in [
        "",
        "example.com/path",
        "user@example.com",
        "user:password@example.com",
        "example.com?query=value",
        "example.com#fragment",
    ] {
        for from_env in [false, true] {
            let mut cmd = command(&dir);
            cmd.arg("logout");
            if from_env {
                cmd.env("TOPK_AUTH_DOMAIN", domain);
            } else {
                cmd.args(["--auth-domain", domain]);
            }
            let output = cmd.output().unwrap();
            assert_eq!(output.status.code(), Some(2), "domain: {domain:?}");
            assert!(String::from_utf8(output.stderr)
                .unwrap()
                .contains("--auth-domain"));
        }
    }
    let output = command(&dir)
        .args(["logout", "--auth-client-id", ""])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.path().join("topk/credentials.toml").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn logout_without_a_session_cleans_up_legacy_api_key() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("topk")).unwrap();
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
    assert!(dir.path().join("topk/session.lock").exists());
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
