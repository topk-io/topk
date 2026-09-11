use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::http::{header, StatusCode};
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;
use url::Url;

use topk::auth::{Auth, Config};

pub struct Server {
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

pub async fn seed(auth: &Auth) {
    let login = auth.login(&[0]).await.unwrap();
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

pub fn response(expires_in: u64, refresh: Option<&str>) -> Value {
    json!({"token_type": "Bearer", "access_token": "access", "refresh_token": refresh, "expires_in": expires_in})
}

pub fn tenant_dir(config_dir: &Path, issuer: &Url) -> PathBuf {
    config_dir
        .join("tenants")
        .join(format!("{:x}", Sha256::digest(issuer.as_str().as_bytes())))
}
