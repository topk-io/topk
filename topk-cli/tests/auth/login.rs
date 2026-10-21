use std::collections::{HashMap, HashSet};
use std::time::Duration;

use oauth2::{PkceCodeChallenge, PkceCodeVerifier};
use test_context::test_context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use url::Url;

use super::common::{response, seed, tenant_dir, AuthTestContext};

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

#[test_context(AuthTestContext)]
#[tokio::test]
async fn unrelated_requests_do_not_finish_login_and_success_follows_persistence(
    ctx: &mut AuthTestContext,
) {
    let auth = ctx.server.auth(ctx.dir.path());
    let login = auth.login(&[0]).await.unwrap();
    let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
    let callback = Url::parse(&params["redirect_uri"]).unwrap();
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["client_id"], ctx.server.config().client_id);
    assert_eq!(params["audience"], ctx.server.config().audience);
    assert_eq!(params["scope"], "openid profile email offline_access");
    assert_eq!(params["prompt"], "login");
    assert_eq!(params["code_challenge_method"], "S256");
    let port = callback.port().unwrap();
    let path = format!("/callback?code=valid&state={}", params["state"]);
    let empty = format!("/callback?code=&state={}", params["state"]);
    let duplicate = format!("{path}&state={}", params["state"]);
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
            let req = ctx.server.request().await;
            let form: HashMap<_, _> = url::form_urlencoded::parse(req.as_bytes())
                .into_owned()
                .collect();
            assert_eq!(form["grant_type"], "authorization_code");
            assert_eq!(form["code"], "valid");
            assert_eq!(form["client_id"], params["client_id"]);
            assert_eq!(form["redirect_uri"], params["redirect_uri"]);
            assert_eq!(
                PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(
                    form["code_verifier"].clone(),
                ))
                .as_str(),
                params["code_challenge"],
            );
            assert!(!tenant_dir(ctx.dir.path(), &ctx.server.url)
                .join("credentials.toml")
                .exists());
            assert!(request(port, "GET", &path)
                .await
                .starts_with("HTTP/1.1 409"));
            ctx.server.reply(200, response(3600, Some("refresh"))).await;
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

#[test_context(AuthTestContext)]
#[tokio::test]
async fn failed_exchange_and_failed_save_never_send_success(ctx: &mut AuthTestContext) {
    for fail_save in [false, true] {
        let auth = ctx.server.auth(ctx.dir.path());
        let login = auth.login(&[0]).await.unwrap();
        let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
        let callback = Url::parse(&params["redirect_uri"]).unwrap();
        let port = callback.port().unwrap();
        let path = format!("/callback?code=valid&state={}", params["state"]);
        if fail_save {
            // A real filesystem failure, without changing the storage implementation.
            std::fs::create_dir_all(
                tenant_dir(ctx.dir.path(), &ctx.server.url).join("credentials.toml"),
            )
            .unwrap();
        }
        let browser = async {
            let exchange = async {
                ctx.server.request().await;
                if fail_save {
                    ctx.server.reply(200, response(3600, Some("refresh"))).await;
                } else {
                    ctx.server
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

#[test_context(AuthTestContext)]
#[tokio::test]
async fn login_replaces_unreadable_credentials(ctx: &mut AuthTestContext) {
    let path = tenant_dir(ctx.dir.path(), &ctx.server.url).join("credentials.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"\xff").unwrap();
    let auth = ctx.server.auth(ctx.dir.path());
    ctx.server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;
    assert_eq!(auth.access_token().await.unwrap(), "access");
}

#[test_context(AuthTestContext)]
#[tokio::test]
async fn cancelling_login_drops_listener(ctx: &mut AuthTestContext) {
    let auth = ctx.server.auth(ctx.dir.path());
    let login = auth.login(&[0]).await.unwrap();
    let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
    let callback = Url::parse(&params["redirect_uri"]).unwrap();
    let addr = ("127.0.0.1", callback.port().unwrap());
    tokio::select! {
        _ = login.finish() => panic!("unexpected callback"),
        _ = sleep(Duration::from_millis(25)) => {},
    }
    assert!(TcpListener::bind(addr).await.is_ok());
}

#[test_context(AuthTestContext)]
#[tokio::test]
async fn authenticated_denial_finishes_without_exchanging_a_token(ctx: &mut AuthTestContext) {
    let auth = ctx.server.auth(ctx.dir.path());
    let login = auth.login(&[0]).await.unwrap();
    let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
    let callback = Url::parse(&params["redirect_uri"]).unwrap();
    let port = callback.port().unwrap();
    let path = format!(
        "/callback?state={}&error=access_denied&error_description=cancelled",
        params["state"]
    );
    let (result, page) = tokio::join!(login.finish(), request(port, "GET", &path));
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("access_denied: cancelled"));
    assert!(page.contains("Login failed"));
    assert!(ctx.server.requests.try_recv().is_err());
}

#[test_context(AuthTestContext)]
#[tokio::test]
async fn concurrent_logins_use_distinct_callback_ports(ctx: &mut AuthTestContext) {
    let auth = ctx.server.auth(ctx.dir.path());
    let logins = futures::future::join_all((0..8).map(|_| auth.login(&[0]))).await;
    let ports: HashSet<_> = logins
        .iter()
        .map(|login| {
            let params: HashMap<_, _> = login
                .as_ref()
                .unwrap()
                .url()
                .query_pairs()
                .into_owned()
                .collect();
            let callback = Url::parse(&params["redirect_uri"]).unwrap();
            assert_eq!(callback.host_str(), Some("127.0.0.1"));
            callback.port().unwrap()
        })
        .collect();
    assert_eq!(ports.len(), logins.len());
}

#[test_context(AuthTestContext)]
#[tokio::test]
async fn login_uses_the_next_port_when_one_is_occupied(ctx: &mut AuthTestContext) {
    let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let auth = ctx.server.auth(ctx.dir.path());
    let login = auth
        .login(&[occupied.local_addr().unwrap().port(), 0])
        .await
        .unwrap();
    let params: HashMap<_, _> = login.url().query_pairs().into_owned().collect();
    let callback = Url::parse(&params["redirect_uri"]).unwrap();
    assert_ne!(
        callback.port().unwrap(),
        occupied.local_addr().unwrap().port()
    );
}
