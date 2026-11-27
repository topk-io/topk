use std::fs::{create_dir_all, File};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tempfile::TempDir;
use tokio::net::TcpListener;

use topk::auth::Auth;
use topk::config::{lock, read, write_secret_file, Config};
use topk::host::DEFAULT_HOST;

use super::common::args;

#[test]
fn replacement_preserves_open_readers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    write_secret_file(&path, "old session").unwrap();
    let mut reader = File::open(&path).unwrap();

    write_secret_file(&path, "new session").unwrap();

    let mut old = String::new();
    reader.read_to_string(&mut old).unwrap();
    assert_eq!(old, "old session");
    assert_eq!(read(&path).unwrap().as_deref(), Some("new session"));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn failed_replacement_cleans_up_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    create_dir_all(&path).unwrap();
    std::fs::write(path.join("existing"), "preserve me").unwrap();

    assert!(write_secret_file(&path, "new session").is_err());

    assert_eq!(
        std::fs::read_to_string(path.join("existing")).unwrap(),
        "preserve me"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn atomic_replacement_never_exposes_partial_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    write_secret_file(&path, "{\"value\":0}").unwrap();
    std::thread::scope(|scope| {
        let path = &path;
        let writer = scope.spawn(move || {
            for n in 1..50 {
                write_secret_file(
                    path,
                    &serde_json::json!({"value": n, "data": "x".repeat(4096)}).to_string(),
                )
                .unwrap();
            }
        });
        for _ in 0..100 {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert!(value["value"].is_number());
        }
        writer.join().unwrap();
    });
}

// Invoked in a separate test process by the parent below.
#[tokio::test]
async fn lock_child() {
    let Some(dir) = std::env::var_os("TOPK_TEST_LOCK_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::write(dir.join("ready"), "").unwrap();
    let _guard = lock(dir.join("shared.lock")).await.unwrap();
    std::fs::write(dir.join("acquired"), "").unwrap();
}

#[tokio::test]
async fn lock_coordinates_separate_processes() {
    let dir = tempfile::tempdir().unwrap();
    let guard = lock(dir.path().join("shared.lock")).await.unwrap();
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "auth::config::lock_child"])
        .env("TOPK_TEST_LOCK_DIR", dir.path())
        .stdout(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !dir.path().join("ready").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(!dir.path().join("acquired").exists());
    drop(guard);
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    assert!(dir.path().join("acquired").exists());
}

#[cfg(unix)]
#[test]
fn write_secret_file_uses_restrictive_permissions() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("credentials.json");

    write_secret_file(&path, "{}").expect("write file");

    let mode = std::fs::metadata(&path)
        .expect("stat file")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[tokio::test]
async fn given_settings_are_used_without_asking_the_host() {
    let dir = TempDir::new().unwrap();
    let config = Config::load(
        &args(&[
            "test",
            "--host",
            "unreachable.invalid",
            "--auth-issuer",
            "https://tenant.test/",
            "--auth-client-id",
            "cli",
            "--auth-audience",
            "https://api.test",
        ]),
        dir.path().to_owned(),
    )
    .await
    .unwrap();
    assert_eq!(config.oauth().client_id, "cli");
    assert!(dir.path().read_dir().unwrap().next().is_none());
}

#[tokio::test]
async fn default_host_uses_the_built_in_settings() {
    // The default host is not asked and nothing is saved.
    let dir = TempDir::new().unwrap();
    let config = Config::load(
        &args(&["test", "--host", DEFAULT_HOST]),
        dir.path().to_owned(),
    )
    .await
    .unwrap();
    assert_eq!(
        config.oauth().issuer.as_str(),
        "https://topk-prod.us.auth0.com/"
    );
    assert!(dir.path().read_dir().unwrap().next().is_none());
}

#[tokio::test]
async fn unreachable_host_falls_back_to_the_built_in_settings() {
    let dir = TempDir::new().unwrap();
    let config = Config::load(
        &args(&["test", "--host", "unreachable.invalid"]),
        dir.path().to_owned(),
    )
    .await
    .unwrap();
    assert_eq!(
        config.oauth().issuer.as_str(),
        "https://topk-prod.us.auth0.com/"
    );
    assert!(dir.path().read_dir().unwrap().next().is_none());
}

#[tokio::test]
async fn host_is_asked_until_login_saves_its_answer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = format!("localhost:{}", listener.local_addr().unwrap().port());
    let asked = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        "/auth/config.json",
        get({
            let asked = asked.clone();
            move || async move {
                asked.fetch_add(1, Ordering::SeqCst);
                r#"{"issuer":"https://tenant.test/","cli_client_id":"cli","audience":"https://api.test"}"#
            }
        }),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let dir = TempDir::new().unwrap();
    let args = args(&["test", "--host", &host, "--https", "false"]);
    let load = || Config::load(&args, dir.path().to_owned());

    let config = load().await.unwrap();
    assert_eq!(config.oauth().client_id, "cli");
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    // A login saves the settings with the profile.
    config.save_profile().unwrap();
    assert_eq!(load().await.unwrap().oauth().client_id, "cli");
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    Auth::new(config).unwrap().logout().await.unwrap();
    load().await.unwrap();
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}
