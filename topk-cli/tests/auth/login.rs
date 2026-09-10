use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use super::*;
use crate::auth::tests::{response, Server, LOGIN};

async fn request(port: u16, method: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream
        .write_all(
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    timeout(Duration::from_secs(5), stream.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}

#[tokio::test]
async fn unrelated_requests_do_not_finish_login_and_success_follows_persistence() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = server.auth(dir.path());
    let _guard = LOGIN.lock().await;
    let login = Login::new(&auth).await.unwrap();
    let port = login.listener.local_addr().unwrap().port();
    let path = format!("/callback?code=valid&state={}", login.state);
    let empty = format!("/callback?code=&state={}", login.state);
    let duplicate = format!("{path}&state={}", login.state);
    let ambiguous = format!("{path}&error=access_denied");
    let mut silent = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let browser = async {
        for (method, path, status) in [
            ("GET", "/favicon.ico", 404),
            ("POST", path.as_str(), 405),
            ("HEAD", path.as_str(), 405),
            ("GET", "/callback?code=bad&state=wrong", 400),
            ("GET", "/callback?error=access_denied&state=wrong", 400),
            ("GET", "/callback?code=bad", 400),
            ("GET", empty.as_str(), 400),
            ("GET", duplicate.as_str(), 400),
            ("GET", ambiguous.as_str(), 400),
        ] {
            assert!(request(port, method, path)
                .await
                .starts_with(&format!("HTTP/1.1 {status}")));
        }
        let callback = request(port, "GET", &path);
        let exchange = async {
            let req = server.request().await;
            assert!(req.contains("code=valid") && req.contains("code_verifier="));
            assert!(auth.store.lock().await.unwrap().load().unwrap().is_none());
            assert!(request(port, "GET", &path)
                .await
                .starts_with("HTTP/1.1 409"));
            server.reply(200, response(3600, Some("refresh"))).await;
        };
        let (page, ()) = tokio::join!(callback, exchange);
        assert!(page.contains("You're logged in"));
        assert_eq!(auth.access_token().await.unwrap(), "access");
    };
    let (result, ()) = tokio::join!(login.finish(), browser);
    result.unwrap();
    assert_eq!(
        timeout(Duration::from_secs(1), silent.read(&mut [0]))
            .await
            .unwrap()
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn failed_exchange_and_failed_save_never_send_success() {
    for fail_save in [false, true] {
        let dir = TempDir::new().unwrap();
        let mut server = Server::new().await;
        let auth = server.auth(dir.path());
        let _guard = LOGIN.lock().await;
        let login = Login::new(&auth).await.unwrap();
        let port = login.listener.local_addr().unwrap().port();
        let path = format!("/callback?code=valid&state={}", login.state);
        if fail_save {
            // A real filesystem failure, without changing the storage implementation.
            std::fs::create_dir(dir.path().join("credentials.toml")).unwrap();
        }
        let browser = async {
            let exchange = async {
                server.request().await;
                if fail_save {
                    server.reply(200, response(3600, Some("refresh"))).await;
                } else {
                    server
                        .reply(400, serde_json::json!({"error":"invalid_grant"}))
                        .await;
                }
            };
            let (page, ()) = tokio::join!(request(port, "GET", &path), exchange);
            assert!(page.contains("Login failed"));
            assert!(!page.contains("You're logged in"));
        };
        let (result, ()) = tokio::join!(login.finish(), browser);
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn storage_failure_is_reported_before_browser_login() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    std::fs::create_dir(dir.path().join("credentials.toml")).unwrap();
    assert!(auth.login().await.is_err());
}

#[tokio::test]
async fn cancelling_login_drops_listener() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    let _guard = LOGIN.lock().await;
    let login = Login::new(&auth).await.unwrap();
    let addr = login.listener.local_addr().unwrap();
    tokio::select! {
        _ = login.finish() => panic!("unexpected callback"),
        _ = sleep(Duration::from_millis(25)) => {},
    }
    assert!(TcpListener::bind(addr).await.is_ok());
}

#[tokio::test]
async fn authenticated_denial_finishes_without_exchanging_a_token() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = server.auth(dir.path());
    let _guard = LOGIN.lock().await;
    let login = Login::new(&auth).await.unwrap();
    let port = login.listener.local_addr().unwrap().port();
    let path = format!(
        "/callback?state={}&error=access_denied&error_description=cancelled",
        login.state
    );
    let (result, page) = tokio::join!(login.finish(), request(port, "GET", &path));
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("access_denied: cancelled"));
    assert!(page.contains("Login failed"));
    assert!(server.requests.try_recv().is_err());
}
