use std::collections::HashMap;
use std::fs::{read_dir, read_to_string, write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{header, StatusCode};
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;
use url::Url;

use super::{Auth, Config, CredentialsStore};

pub(super) struct Server {
    pub url: Url,
    pub requests: mpsc::Receiver<String>,
    pub replies: mpsc::Sender<(u16, String)>,
    _shutdown: oneshot::Sender<()>,
}

impl Server {
    pub async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        let (requests_tx, requests) = mpsc::channel(32);
        let (replies, replies_rx) = mpsc::channel::<(u16, String)>(32);
        let replies_rx = Arc::new(Mutex::new(replies_rx));
        let app = Router::new().route(
            "/oauth/token",
            post(move |body: String| {
                let requests_tx = requests_tx.clone();
                let replies_rx = replies_rx.clone();
                async move {
                    let mut replies_rx = replies_rx.lock().await;
                    requests_tx.send(body).await.unwrap();
                    let (status, body) = replies_rx.recv().await.unwrap_or((503, String::new()));
                    (
                        StatusCode::from_u16(status).unwrap(),
                        [(header::CONTENT_TYPE, "application/json")],
                        body,
                    )
                }
            }),
        );
        let (shutdown, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            url,
            requests,
            replies,
            _shutdown: shutdown,
        }
    }

    pub fn config(&self) -> Config {
        Config {
            issuer: self.url.clone(),
            client_id: "test-client".into(),
            audience: "https://api.test".into(),
            store: CredentialsStore::File,
        }
    }

    pub fn auth(&self, config_dir: &Path) -> Auth {
        Auth::new(&self.config(), config_dir.to_owned()).unwrap()
    }

    pub async fn reply(&self, status: u16, body: Value) {
        self.replies.send((status, body.to_string())).await.unwrap();
    }

    pub async fn request(&mut self) -> String {
        timeout(Duration::from_secs(5), self.requests.recv())
            .await
            .unwrap()
            .unwrap()
    }
}

// Serialize logins sharing the three registered callback ports.
pub(super) static LOGIN: Mutex<()> = Mutex::const_new(());

pub(super) async fn seed(auth: &Auth) {
    let _guard = LOGIN.lock().await;
    let login = auth.login().await.unwrap();
    let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
    let mut callback = Url::parse(&params["redirect_uri"]).unwrap();
    callback
        .query_pairs_mut()
        .append_pair("code", "initial")
        .append_pair("state", &params["state"]);
    let (result, page) = tokio::join!(login.finish(), reqwest::get(callback));
    result.unwrap();
    assert!(page
        .unwrap()
        .text()
        .await
        .unwrap()
        .contains("You're logged in"));
}

pub(super) fn response(expires_in: u64, refresh: Option<&str>) -> Value {
    json!({"access_token": "access", "refresh_token": refresh, "expires_in": expires_in})
}

#[tokio::test]
async fn concurrent_refresh_happens_once_and_preserves_refresh_token() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = Arc::new(server.auth(dir.path()));
    server.reply(200, response(0, Some("refresh"))).await;
    seed(&auth).await;
    server.request().await;
    server.reply(200, response(3600, None)).await;
    let mut jobs = Vec::new();
    for _ in 0..12 {
        let auth = auth.clone();
        jobs.push(tokio::spawn(
            async move { auth.access_token().await.unwrap() },
        ));
    }
    for job in jobs {
        assert_eq!(job.await.unwrap(), "access");
    }
    let request = server.request().await;
    assert!(
        request.contains("grant_type=refresh_token") && request.contains("refresh_token=refresh")
    );
    assert!(server.requests.try_recv().is_err());
    assert_eq!(
        auth.store
            .lock()
            .await
            .unwrap()
            .load()
            .unwrap()
            .unwrap()
            .refresh_token
            .as_deref(),
        Some("refresh")
    );
}

#[tokio::test]
async fn logout_waits_for_refresh_and_removes_rotated_session() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = Arc::new(server.auth(dir.path()));
    server.reply(200, response(0, Some("refresh"))).await;
    seed(&auth).await;
    server.request().await;
    let refresh_auth = auth.clone();
    let refresh = tokio::spawn(async move { refresh_auth.access_token().await });
    server.request().await;
    let logout_auth = auth.clone();
    let mut logout = tokio::spawn(async move { logout_auth.logout().await });
    assert!(timeout(Duration::from_millis(75), &mut logout)
        .await
        .is_err());
    server.reply(200, response(3600, Some("rotated"))).await;
    refresh.await.unwrap().unwrap();
    logout.await.unwrap().unwrap();
    assert!(auth
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("not logged in"));
}

#[tokio::test]
async fn invalid_grant_clears_session_and_other_errors_preserve_it() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = server.auth(dir.path());
    for (status, body, retained, message) in [
        (503, "<html>unavailable</html>", true, "503"),
        (400, r#"{"error":"server_error"}"#, true, "server_error"),
        (
            400,
            r#"{"error":"invalid_grant"}"#,
            false,
            "session expired",
        ),
    ] {
        server.reply(200, response(0, Some("refresh"))).await;
        seed(&auth).await;
        server.request().await;
        server.replies.send((status, body.into())).await.unwrap();
        let error = auth.access_token().await.unwrap_err();
        assert!(format!("{error:#}").contains(message));
        server.request().await;
        assert_eq!(
            auth.store.lock().await.unwrap().load().unwrap().is_some(),
            retained
        );
    }
}

#[tokio::test]
async fn request_timeout_releases_refresh_lock_and_preserves_session() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = Arc::new(server.auth(dir.path()));
    server.reply(200, response(0, Some("refresh"))).await;
    seed(&auth).await;
    server.request().await;
    let refresh_auth = auth.clone();
    let refresh = tokio::spawn(async move { refresh_auth.access_token().await });
    server.request().await;
    // The production client has a 30-second request deadline.
    let result = timeout(Duration::from_secs(35), refresh)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    assert!(auth.store.lock().await.unwrap().load().unwrap().is_some());
    timeout(Duration::from_secs(1), auth.logout())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn configuration_mismatches_require_login_and_preserve_existing_credentials() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;
    for changed in ["issuer", "client", "audience"] {
        let mut config = server.config();
        match changed {
            "issuer" => config.issuer = Url::parse("https://other.test/").unwrap(),
            "client" => config.client_id = "other-client".into(),
            _ => config.audience = "https://other-api.test".into(),
        }
        let other = Auth::new(&config, dir.path().to_owned()).unwrap();
        assert!(other
            .access_token()
            .await
            .unwrap_err()
            .to_string()
            .contains("authentication configuration mismatch"));
        other.store.lock().await.unwrap().prepare().unwrap();
        assert_eq!(auth.access_token().await.unwrap(), "access");
    }
}

#[tokio::test]
async fn existing_file_session_keeps_backend_and_logout_uses_it() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;
    // Even forcing keyring for new sessions must not hide this existing file session.
    let reopened = Auth::new(
        &Config {
            store: CredentialsStore::Keyring,
            ..server.config()
        },
        dir.path().to_owned(),
    )
    .unwrap();
    reopened.store.lock().await.unwrap().prepare().unwrap();
    assert_eq!(reopened.access_token().await.unwrap(), "access");
    reopened.logout().await.unwrap();
    assert!(auth.store.lock().await.unwrap().load().unwrap().is_none());
}

#[tokio::test]
async fn login_replaces_the_single_stored_session() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;

    let config = Config {
        audience: "https://other-api.test".into(),
        ..server.config()
    };
    let key = config.oauth().key().unwrap();
    let other = Auth::new(&config, dir.path().to_owned()).unwrap();
    server
        .reply(
            200,
            json!({"access_token": "replacement", "expires_in": 3600}),
        )
        .await;
    seed(&other).await;
    assert_eq!(other.access_token().await.unwrap(), "replacement");
    assert!(auth
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("authentication configuration mismatch"));
    let record: toml::Table =
        toml::from_str(&read_to_string(dir.path().join("credentials.toml")).unwrap()).unwrap();
    assert_eq!(record["store_key"].as_str(), Some(key.as_str()));
    assert_eq!(record["access_token"].as_str(), Some("replacement"));
    let config: toml::Table =
        toml::from_str(&read_to_string(dir.path().join("config.toml")).unwrap()).unwrap();
    assert_eq!(config["store"].as_str(), Some("file"));
    assert_eq!(config.len(), 1);

    let mut files: Vec<_> = read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    files.sort();
    assert_eq!(files, ["config.toml", "credentials.toml", "session.lock"]);

    // Logout removes the current session even when invoked with the old configuration.
    auth.logout().await.unwrap();
    assert!(!dir.path().join("credentials.toml").exists());
    assert!(!dir.path().join("config.toml").exists());
    assert!(other
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("not logged in"));
}

#[tokio::test]
async fn login_and_logout_preserve_unrelated_config_and_remove_legacy_api_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    write(
        &path,
        "api_key = 'legacy-secret'\n[preferences]\ncolor = false\n",
    )
    .unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;
    let config: toml::Table = toml::from_str(&read_to_string(&path).unwrap()).unwrap();
    assert!(!config.contains_key("api_key"));
    assert_eq!(config["store"].as_str(), Some("file"));
    assert_eq!(config["preferences"]["color"].as_bool(), Some(false));
    auth.logout().await.unwrap();
    let config: toml::Table = toml::from_str(&read_to_string(path).unwrap()).unwrap();
    assert_eq!(config.len(), 1);
    assert_eq!(config["preferences"]["color"].as_bool(), Some(false));
    assert!(!dir.path().join("credentials.toml").exists());
}

#[tokio::test]
async fn logout_without_a_session_removes_legacy_api_key_and_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    write(
        &path,
        "api_key = 'legacy-secret'\n[preferences]\ncolor = false\n",
    )
    .unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    auth.logout().await.unwrap();
    auth.logout().await.unwrap();
    let config: toml::Table = toml::from_str(&read_to_string(path).unwrap()).unwrap();
    assert_eq!(config.len(), 1);
    assert_eq!(config["preferences"]["color"].as_bool(), Some(false));
    assert!(!dir.path().join("credentials.toml").exists());
}

#[tokio::test]
async fn file_storage_handles_sessions_larger_than_windows_credential_limit() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = Auth::new(
        &Config {
            store: if cfg!(windows) {
                CredentialsStore::Auto
            } else {
                CredentialsStore::File
            },
            ..server.config()
        },
        dir.path().to_owned(),
    )
    .unwrap();
    // The token alone reaches the Windows limit at 1,280 UTF-16 code units;
    // metadata and the refresh token also count toward that limit.
    for size in [1_279, 1_280, 1_281, 16_384] {
        let access_token = "a".repeat(size);
        let refresh_token = "r".repeat(size);
        server
            .reply(
                200,
                json!({
                    "access_token": access_token,
                    "refresh_token": refresh_token,
                    "expires_in": 3600,
                }),
            )
            .await;
        seed(&auth).await;
        assert_eq!(auth.access_token().await.unwrap(), access_token);
        let session = auth.store.lock().await.unwrap().load().unwrap().unwrap();
        assert_eq!(
            session.refresh_token.as_deref(),
            Some(refresh_token.as_str())
        );
    }
    auth.logout().await.unwrap();
    assert!(!dir.path().join("credentials.toml").exists());
}

#[cfg(windows)]
#[tokio::test]
async fn windows_rejects_keyring_before_browser_login() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = Auth::new(
        &Config {
            store: CredentialsStore::Keyring,
            ..server.config()
        },
        dir.path().to_owned(),
    )
    .unwrap();
    let error = match auth.login().await {
        Ok(_) => panic!("unsupported keyring storage should fail before browser login"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("use file storage"));
    assert!(!dir.path().join("config.toml").exists());
}

#[tokio::test]
async fn invalid_storage_configuration_is_rejected_without_modifying_files() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    let config_path = dir.path().join("config.toml");
    let credentials_path = dir.path().join("credentials.toml");
    write(&credentials_path, "existing credentials").unwrap();
    for config in ["store = 'unknown'\n", "store = 123\n", "store = 'auto'\n"] {
        write(&config_path, config).unwrap();
        assert!(auth
            .logout()
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid config.toml"));
        assert_eq!(read_to_string(&config_path).unwrap(), config);
        assert_eq!(
            read_to_string(&credentials_path).unwrap(),
            "existing credentials"
        );
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires an isolated, unlocked Secret Service keyring"]
async fn file_and_keyring_support_login_refresh_and_logout() {
    for kind in [
        CredentialsStore::File,
        CredentialsStore::Keyring,
        CredentialsStore::Auto,
    ] {
        let dir = TempDir::new().unwrap();
        let mut server = Server::new().await;
        let auth = Auth::new(
            &Config {
                store: kind,
                ..server.config()
            },
            dir.path().to_owned(),
        )
        .unwrap();
        // Exercise payloads larger than a small API key in both backends.
        let refresh_token = "r".repeat(16_384);
        server
            .reply(
                200,
                json!({
                    "access_token": "a".repeat(16_384),
                    "refresh_token": refresh_token,
                    "expires_in": 0,
                }),
            )
            .await;
        seed(&auth).await;
        assert!(server
            .request()
            .await
            .contains("grant_type=authorization_code"));
        let config: toml::Table =
            toml::from_str(&read_to_string(dir.path().join("config.toml")).unwrap()).unwrap();
        assert_eq!(
            config["store"].as_str(),
            Some(if matches!(kind, CredentialsStore::File) {
                "file"
            } else {
                "keyring"
            }),
        );
        assert_eq!(config.len(), 1);
        assert_eq!(
            dir.path().join("credentials.toml").exists(),
            matches!(kind, CredentialsStore::File),
        );
        server
            .reply(
                200,
                json!({
                    "access_token": "renewed-access",
                    "refresh_token": "rotated-refresh",
                    "expires_in": 3600,
                }),
            )
            .await;
        let auth = Arc::new(auth);
        let mut jobs = Vec::new();
        for _ in 0..8 {
            let auth = auth.clone();
            jobs.push(tokio::spawn(
                async move { auth.access_token().await.unwrap() },
            ));
        }
        for job in jobs {
            assert_eq!(job.await.unwrap(), "renewed-access");
        }
        let request = server.request().await;
        assert!(request.contains("grant_type=refresh_token"));
        assert!(request.contains(&format!("refresh_token={refresh_token}")));
        let session = auth.store.lock().await.unwrap().load().unwrap().unwrap();
        assert_eq!(session.refresh_token.as_deref(), Some("rotated-refresh"));
        assert_eq!(auth.access_token().await.unwrap(), "renewed-access");
        assert!(server.requests.try_recv().is_err());
        auth.logout().await.unwrap();
        auth.logout().await.unwrap();
        assert!(auth.store.lock().await.unwrap().load().unwrap().is_none());
        assert!(!dir.path().join("credentials.toml").exists());
        assert!(!dir.path().join("config.toml").exists());
        eprintln!(
            "{kind:?}: login, large payload, concurrent refresh rotation, cached token, and logout passed"
        );
    }
}
