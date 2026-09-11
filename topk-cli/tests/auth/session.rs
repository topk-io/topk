use std::fs::{read_dir, read_to_string, write};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;

use topk::auth::{Auth, Config};

use super::common::{response, seed, tenant_dir, Server};

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
    let credentials: toml::Table = toml::from_str(
        &read_to_string(tenant_dir(dir.path(), &server.url).join("credentials.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(credentials["refresh_token"].as_str(), Some("refresh"));
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
        (
            503,
            "<html>unavailable</html>",
            true,
            "Failed to parse server response",
        ),
        (400, r#"{"error":"server_error"}"#, true, "server_error"),
        (
            200,
            r#"{"access_token":"invalid","expires_in":3600}"#,
            true,
            "Failed to parse server response",
        ),
        (
            200,
            r#"{"access_token":"invalid","token_type":"Bearer"}"#,
            true,
            "token response is missing expires_in",
        ),
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
            tenant_dir(dir.path(), &server.url)
                .join("credentials.toml")
                .exists(),
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
    assert!(tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
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
    for changed in ["client", "audience"] {
        let mut config = server.config();
        match changed {
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
        drop(other.login(&[0]).await.unwrap());
        assert_eq!(auth.access_token().await.unwrap(), "access");
    }
}

#[tokio::test]
async fn session_survives_reopening_and_logout_removes_it() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;
    let reopened = server.auth(dir.path());
    assert_eq!(reopened.access_token().await.unwrap(), "access");
    reopened.logout().await.unwrap();
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
}

#[tokio::test]
async fn login_replaces_the_session_for_the_same_issuer() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    server.reply(200, response(3600, Some("refresh"))).await;
    seed(&auth).await;

    let config = Config {
        audience: "https://other-api.test".into(),
        ..server.config()
    };
    let other = Auth::new(&config, dir.path().to_owned()).unwrap();
    server
        .reply(
            200,
            json!({"token_type": "Bearer", "access_token": "replacement", "expires_in": 3600}),
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
    let record: toml::Table = toml::from_str(
        &read_to_string(tenant_dir(dir.path(), &server.url).join("credentials.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(record["access_token"].as_str(), Some("replacement"));
    assert_eq!(
        record["client_id"].as_str(),
        Some(config.client_id.as_str())
    );
    assert_eq!(record["audience"].as_str(), Some(config.audience.as_str()));
    assert!(!record.contains_key("store_key"));
    assert!(!dir.path().join("config.toml").exists());

    let mut files: Vec<_> = read_dir(tenant_dir(dir.path(), &server.url))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    files.sort();
    assert_eq!(files, ["credentials.toml", "session.lock"]);

    // Logout removes the current session even when invoked with the old configuration.
    auth.logout().await.unwrap();
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
    assert!(!dir.path().join("config.toml").exists());
    assert!(other
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("not logged in"));
}

#[tokio::test]
async fn login_preserves_preferences_and_logout_removes_both_files() {
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
    assert_eq!(config["preferences"]["color"].as_bool(), Some(false));
    auth.logout().await.unwrap();
    assert!(!path.exists());
    assert!(tenant_dir(dir.path(), &server.url)
        .join("session.lock")
        .exists());
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
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
    assert!(!path.exists());
    assert!(tenant_dir(dir.path(), &server.url)
        .join("session.lock")
        .exists());
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
}

#[tokio::test]
async fn file_storage_handles_large_sessions() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    for size in [1_024, 4_096, 16_384] {
        let access_token = "a".repeat(size);
        let refresh_token = "r".repeat(size);
        server
            .reply(
                200,
                json!({
                    "token_type": "Bearer",
                    "access_token": access_token,
                    "refresh_token": refresh_token,
                    "expires_in": 3600,
                }),
            )
            .await;
        seed(&auth).await;
        assert_eq!(auth.access_token().await.unwrap(), access_token);
        let session: toml::Table = toml::from_str(
            &read_to_string(tenant_dir(dir.path(), &server.url).join("credentials.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            session["refresh_token"].as_str(),
            Some(refresh_token.as_str())
        );
    }
    auth.logout().await.unwrap();
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
}

#[tokio::test]
async fn logout_removes_malformed_config_and_credentials() {
    let dir = TempDir::new().unwrap();
    let server = Server::new().await;
    let auth = server.auth(dir.path());
    let config_path = dir.path().join("config.toml");
    let credentials_path = tenant_dir(dir.path(), &server.url).join("credentials.toml");
    std::fs::create_dir_all(credentials_path.parent().unwrap()).unwrap();
    for config in [b"invalid [".as_slice(), b"= true", b"[unclosed", b"\xff"] {
        write(&config_path, config).unwrap();
        write(&credentials_path, "existing credentials").unwrap();
        auth.logout().await.unwrap();
        auth.logout().await.unwrap();
        assert!(!config_path.exists());
        assert!(!credentials_path.exists());
        assert!(tenant_dir(dir.path(), &server.url)
            .join("session.lock")
            .exists());
    }
}

#[tokio::test]
async fn file_storage_supports_login_refresh_and_logout() {
    let dir = TempDir::new().unwrap();
    let mut server = Server::new().await;
    let auth = server.auth(dir.path());
    // Exercise payloads larger than a small API key.
    let refresh_token = "r".repeat(16_384);
    server
        .reply(
            200,
            json!({
                "token_type": "Bearer",
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
    assert!(!dir.path().join("config.toml").exists());
    assert!(tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());

    server
        .reply(
            200,
            json!({
                "token_type": "Bearer",
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
    let session: toml::Table = toml::from_str(
        &read_to_string(tenant_dir(dir.path(), &server.url).join("credentials.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(session["refresh_token"].as_str(), Some("rotated-refresh"));
    assert_eq!(auth.access_token().await.unwrap(), "renewed-access");
    assert!(server.requests.try_recv().is_err());
    auth.logout().await.unwrap();
    auth.logout().await.unwrap();
    assert!(auth
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("not logged in"));
    assert!(!tenant_dir(dir.path(), &server.url)
        .join("credentials.toml")
        .exists());
    assert!(!dir.path().join("config.toml").exists());
}

#[tokio::test]
async fn switching_issuers_reuses_sessions() {
    let dir = TempDir::new().unwrap();
    let mut first = Server::new().await;
    let mut second = Server::new().await;
    let first_auth = first.auth(dir.path());
    let second_auth = second.auth(dir.path());
    for (server, auth, token) in [
        (&first, &first_auth, "first-access"),
        (&second, &second_auth, "second-access"),
    ] {
        let mut reply = response(3600, Some("refresh"));
        reply["access_token"] = json!(token);
        server.reply(200, reply).await;
        seed(auth).await;
    }
    first.request().await;
    second.request().await;
    for _ in 0..2 {
        assert_eq!(
            first.auth(dir.path()).access_token().await.unwrap(),
            "first-access"
        );
        assert_eq!(
            second.auth(dir.path()).access_token().await.unwrap(),
            "second-access"
        );
    }
    assert!(first.requests.try_recv().is_err());
    assert!(second.requests.try_recv().is_err());
}

#[tokio::test]
async fn logout_only_removes_the_current_issuer_session() {
    let dir = TempDir::new().unwrap();
    let first = Server::new().await;
    let second = Server::new().await;
    let first_auth = first.auth(dir.path());
    let second_auth = second.auth(dir.path());
    for (server, auth, token) in [
        (&first, &first_auth, "first-access"),
        (&second, &second_auth, "second-access"),
    ] {
        let mut reply = response(3600, Some("refresh"));
        reply["access_token"] = json!(token);
        server.reply(200, reply).await;
        seed(auth).await;
    }
    first_auth.logout().await.unwrap();
    assert!(first_auth
        .access_token()
        .await
        .unwrap_err()
        .to_string()
        .contains("not logged in"));
    assert_eq!(second_auth.access_token().await.unwrap(), "second-access");
    assert!(tenant_dir(dir.path(), &first.url)
        .join("session.lock")
        .exists());
    assert!(tenant_dir(dir.path(), &second.url)
        .join("credentials.toml")
        .exists());
}

#[tokio::test]
async fn refreshing_one_issuer_does_not_block_another() {
    let dir = TempDir::new().unwrap();
    let mut first = Server::new().await;
    let mut second = Server::new().await;
    let first_auth = Arc::new(first.auth(dir.path()));
    let second_auth = second.auth(dir.path());
    first.reply(200, response(0, Some("first-refresh"))).await;
    seed(&first_auth).await;
    first.request().await;
    second.reply(200, response(0, Some("second-refresh"))).await;
    seed(&second_auth).await;
    second.request().await;

    let refreshing_auth = first_auth.clone();
    let refresh = tokio::spawn(async move { refreshing_auth.access_token().await });
    assert!(first
        .request()
        .await
        .contains("refresh_token=first-refresh"));
    second
        .reply(200, response(3600, Some("second-rotated")))
        .await;
    assert_eq!(
        timeout(Duration::from_secs(2), second_auth.access_token())
            .await
            .unwrap()
            .unwrap(),
        "access"
    );
    assert!(second
        .request()
        .await
        .contains("refresh_token=second-refresh"));
    assert!(!refresh.is_finished());

    first
        .reply(200, response(3600, Some("first-rotated")))
        .await;
    assert_eq!(refresh.await.unwrap().unwrap(), "access");
    for (server, token) in [(&first, "first-rotated"), (&second, "second-rotated")] {
        let credentials: toml::Table = toml::from_str(
            &read_to_string(tenant_dir(dir.path(), &server.url).join("credentials.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(credentials["refresh_token"].as_str(), Some(token));
    }
}
