use std::future::{Future, IntoFuture};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{serve, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use oauth2::CsrfToken;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Complete the first valid callback, then report its outcome to the browser.
/// The server stays scoped to this future so cancellation releases the listener.
pub(super) async fn run<F, Fut, T>(
    listener: TcpListener,
    state: CsrfToken,
    duration: Duration,
    complete: F,
) -> Result<T>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let (app, mut receiver) = router(state);
    let (shutdown, stopped) = oneshot::channel::<()>();
    let server = serve(listener, app)
        .with_graceful_shutdown(async {
            // Sender drop also shuts down connections when this future is cancelled.
            let _ = stopped.await;
        })
        .into_future();
    tokio::pin!(server);

    let login = async {
        let Callback { code, response } = timeout(duration, receiver.recv())
            .await
            .context("timed out waiting for the browser login")?
            .context("login callback server stopped")?;
        drop(receiver);
        let result = match code {
            Ok(code) => complete(code).await,
            Err(error) => Err(error),
        };
        let _ = response.send(result.is_ok());
        result
    };
    let result = tokio::select! {
        result = login => result,
        result = &mut server => {
            result.context("serving the login callback")?;
            bail!("login callback server stopped");
        },
    };
    drop(shutdown);
    // Flush the final response without allowing a slow browser to delay exit indefinitely.
    let _ = timeout(SHUTDOWN_TIMEOUT, &mut server).await;
    result
}

fn router(state: CsrfToken) -> (Router, mpsc::Receiver<Callback>) {
    let (callbacks, receiver) = mpsc::channel(1);
    let router = Router::new()
        .route(
            "/callback",
            get(callback).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .with_state(CallbackState { state, callbacks });
    (router, receiver)
}

#[derive(Deserialize)]
struct CallbackParams {
    state: String,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Clone)]
struct CallbackState {
    state: CsrfToken,
    callbacks: mpsc::Sender<Callback>,
}

/// A validated callback whose browser response waits for credential persistence.
struct Callback {
    code: Result<String>,
    response: oneshot::Sender<bool>,
}

async fn callback(
    State(state): State<CallbackState>,
    Query(params): Query<CallbackParams>,
) -> Response {
    if CsrfToken::new(params.state) != state.state {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let code = match (params.code, params.error) {
        (Some(code), None) if !code.is_empty() => Ok(code),
        (None, Some(error)) => Err(anyhow!(
            "{error}: {}",
            params.error_description.unwrap_or_default()
        )),
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (response, received) = oneshot::channel();
    if state
        .callbacks
        .try_send(Callback { code, response })
        .is_err()
    {
        return StatusCode::CONFLICT.into_response();
    }
    page(received.await.unwrap_or(false))
}

fn page(ok: bool) -> Response {
    let (status, title, tone, body) = if ok {
        (
            StatusCode::OK,
            "You're logged in",
            "fg",
            "You can close this tab and return to the terminal.",
        )
    } else {
        (
            StatusCode::BAD_REQUEST,
            "Login failed",
            "err",
            "Could not complete login. Return to the terminal for more information.",
        )
    };

    let html = include_str!("../../assets/callback.html")
        .replace(
            "{{favicon}}",
            &STANDARD.encode(include_bytes!("../../assets/favicon.png")),
        )
        .replace(
            "{{logo}}",
            &include_str!("../../assets/logo.svg").replace("<svg ", "<svg class=\"logo\" "),
        )
        .replace("{{tone}}", tone)
        .replace("{{title}}", title)
        .replace("{{body}}", body);
    (status, [(header::CACHE_CONTROL, "no-store")], Html(html)).into_response()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use oauth2::CsrfToken;
    use tokio::net::{TcpListener, TcpStream};

    use super::run;

    #[tokio::test]
    async fn timeout_releases_listener_even_with_a_silent_connection() {
        for connected in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let _connection = if connected {
                Some(TcpStream::connect(addr).await.unwrap())
            } else {
                None
            };
            let error = run::<_, _, ()>(
                listener,
                CsrfToken::new("test-state".into()),
                Duration::from_millis(50),
                |_| async {
                    unreachable!("unexpected callback");
                },
            )
            .await
            .unwrap_err();
            assert!(error.to_string().contains("timed out"));
            assert!(TcpListener::bind(addr).await.is_ok());
        }
    }
}
