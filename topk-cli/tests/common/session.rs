use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use chrono::{TimeDelta, Utc};
use clap::{Args, FromArgMatches};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use test_context::AsyncTestContext;
use uuid::Uuid;

use topk::auth::{OAuthConfig, Session};
use topk::config::Config;
use topk::host::Host;
use topk::ProjectId;

/// Credentials of the e2e user for real tenants; unset means the emulator issuer.
const TEST_USER: &str = "TOPK_TEST_USER";
const TEST_PASSWORD: &str = "TOPK_TEST_PASSWORD";
/// Auth0 database connection the e2e user belongs to.
const REALM: &str = "Username-Password-Authentication";
/// Reuse a minted session while it has at least this long to live.
const SESSION_MIN_REMAINING: TimeDelta = TimeDelta::minutes(15);
/// Names of the resources this crate creates.
const PROJECT_PREFIX: &str = "it-";
/// The project that data-plane e2e tests run in. Long-lived and found by name, so
/// every test process shares it.
const E2E_PROJECT: &str = "cli-e2e";

/// A config directory of its own, logged in as `topk login` would leave it, so
/// tests never touch the developer's sessions.
pub struct CliTestContext {
    /// Removed with the context; the process env points at it (see `logged_out`).
    _home: TempDir,
    projects: Vec<ProjectId>,
}

impl CliTestContext {
    pub fn logged_out() -> Self {
        assert!(
            env::var("TOPK_AUTH_ISSUER").is_ok_and(|issuer| !issuer.is_empty()),
            "TOPK_AUTH_ISSUER not set: management tests need an issuer to log in to"
        );
        let home = tempfile::tempdir().unwrap();
        // Point this process at the temporary home too, so `topk::config::Config::dir()`
        // resolves the same directory the spawned CLI will use. Sound under
        // nextest, which runs every test in a process of its own.
        env::set_var("HOME", home.path());
        env::set_var("XDG_CONFIG_HOME", home.path());
        CliTestContext {
            _home: home,
            projects: Vec::new(),
        }
    }

    pub async fn login(&self) {
        match (env::var(TEST_USER), env::var(TEST_PASSWORD)) {
            (Ok(user), Ok(password)) if !user.is_empty() && !password.is_empty() => {
                self.login_with_password(&user, &password).await
            }
            _ => self.login_without_browser().await,
        }
    }

    /// `topk login --no-browser` against the emulator issuer, which redirects
    /// straight to the hosted page that shows the code to paste back.
    async fn login_without_browser(&self) {
        let mut child = self
            .command(&["login", "--no-browser"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn topk login");
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let url = stderr
            .by_ref()
            .lines()
            .map(Result::unwrap)
            .find(|line| line.starts_with("http"))
            .expect("login url");
        let page = reqwest::get(url).await.expect("follow login url");
        assert!(page.status().is_success(), "code page {}", page.status());
        let (_, code) = page
            .url()
            .query_pairs()
            .find(|(key, _)| key == "code")
            .expect("code in the page url");
        writeln!(child.stdin.take().unwrap(), "{code}").expect("paste the code");
        let mut rest = String::new();
        stderr.read_to_string(&mut rest).unwrap();
        assert!(
            child.wait().unwrap().success(),
            "topk login failed:\n{rest}"
        );
        assert!(rest.contains("Logged in"), "{rest}");
    }

    /// A real tenant has no scriptable login page: mint the session with the
    /// password grant of the tenant's e2e CLI application and save it as
    /// `topk login` does, so every command runs as after a login.
    ///
    /// One grant per machine while the session is fresh, not one per test:
    /// nextest runs each test in its own process, and the tenant counts every
    /// rejected grant as a failed login, which can block the e2e user.
    async fn login_with_password(&self, user: &str, password: &str) {
        let home = config(Config::dir().expect("config directory"));
        let oauth = home.oauth();
        let cache = config(machine_dir(&format!(
            "session{}{}{user}",
            oauth.client_id, oauth.audience
        )));
        let session = {
            let cached = cache.session().await.expect("lock cached session");
            let fresh = cached.load().expect("cached session").filter(|session| {
                session.expires_at as i64 > (Utc::now() + SESSION_MIN_REMAINING).timestamp()
            });
            match fresh {
                Some(session) => session,
                None => {
                    let session = grant(oauth, user, password).await;
                    cached.save(session.clone()).expect("cache session");
                    session
                }
            }
        };
        home.session()
            .await
            .expect("lock session")
            .save(session)
            .expect("save session");
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_topk"));
        cmd.args(args).env("TOPK_AUTH_CALLBACK_PORTS", "0");
        cmd
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run topk")
    }

    pub fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "`topk {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    pub fn fails(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            !out.status.success(),
            "`topk {}` should have failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stdout)
        );
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    /// `-o json` output, one object per line.
    pub fn json(&self, args: &[&str]) -> Vec<Value> {
        let args: Vec<&str> = args.iter().copied().chain(["-o", "json"]).collect();
        self.ok(&args)
            .lines()
            .map(|line| serde_json::from_str(line).expect("json line"))
            .collect()
    }

    /// Creates a project that teardown deletes.
    pub fn create_project(&mut self, name: &str) -> Value {
        let project = self
            .json(&["project", "create", name])
            .pop()
            .expect("created project");
        self.projects
            .push(project["project_id"].as_str().unwrap().parse().unwrap());
        project
    }

    pub fn select_e2e_project(&self) -> ProjectId {
        let id = self.e2e_project();
        self.ok(&["project", "select", id.as_str()]);
        id
    }

    /// The id of the shared [`E2E_PROJECT`], created on first use. Looked up once
    /// per machine and tenant: nextest runs every test in a process of its own.
    fn e2e_project(&self) -> ProjectId {
        let dir = machine_dir(&format!(
            "project{}{}{}",
            env::var("TOPK_AUTH_ISSUER").unwrap_or_default(),
            env::var("TOPK_HOST").unwrap_or_default(),
            env::var(TEST_USER).unwrap_or_default()
        ));
        let lock = File::create(dir.join("lock")).unwrap();
        lock.lock().unwrap();
        let cached = dir.join("project_id");
        if let Ok(id) = fs::read_to_string(&cached) {
            return id.parse().expect("cached project id");
        }
        let project = self
            .json(&["project", "list"])
            .into_iter()
            .find(|project| project["name"] == E2E_PROJECT)
            .unwrap_or_else(|| {
                self.json(&["project", "create", E2E_PROJECT])
                    .pop()
                    .expect("created project")
            });
        let id = project["project_id"].as_str().unwrap();
        fs::write(&cached, id).unwrap();
        id.parse().expect("project id")
    }
}

impl AsyncTestContext for CliTestContext {
    async fn setup() -> Self {
        let ctx = CliTestContext::logged_out();
        ctx.login().await;
        ctx
    }

    async fn teardown(self) {
        for id in &self.projects {
            let out = self.run(&["project", "delete", "--yes", id.as_str()]);
            if !out.status.success() {
                println!(
                    "Teardown error deleting project {id:?}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
}

pub fn unique_name(prefix: &str) -> String {
    format!(
        "{PROJECT_PREFIX}{prefix}-{}",
        &Uuid::new_v4().simple().to_string()[..8]
    )
}

/// The password grant `topk login`'s session would come from on a real tenant.
async fn grant(oauth: &OAuthConfig, user: &str, password: &str) -> Session {
    let response = reqwest::Client::new()
        .post(oauth.issuer.join("oauth/token").unwrap())
        .form(&[
            (
                "grant_type",
                "http://auth0.com/oauth/grant-type/password-realm",
            ),
            ("realm", REALM),
            ("username", user),
            ("password", password),
            ("client_id", &oauth.client_id),
            ("audience", &oauth.audience),
            ("scope", "openid profile email offline_access"),
        ])
        .send()
        .await
        .expect("token request");
    assert!(
        response.status().is_success(),
        "password grant failed: {}",
        response.text().await.unwrap()
    );
    let token: Token = response.json().await.expect("token response");
    Session {
        access_token: token.access_token,
        refresh_token: Some(token.refresh_token),
        expires_at: Utc::now().timestamp() as u64 + token.expires_in,
    }
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

/// The configuration the spawned CLI reads from this environment, over `dir`.
fn config(dir: PathBuf) -> Config {
    let matches = OAuthConfig::augment_args(Host::augment_args(clap::Command::new("topk")))
        .get_matches_from(["topk"]);
    Config::new(
        Host::from_arg_matches(&matches).expect("host from the environment"),
        OAuthConfig::from_arg_matches(&matches).expect("auth from the environment"),
        dir,
    )
}

/// A directory shared by every test process on this machine for `key`.
fn machine_dir(key: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("topk-cli-tests-{:x}", Sha256::digest(key)));
    fs::create_dir_all(&dir).unwrap();
    dir
}
