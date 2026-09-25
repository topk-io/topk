use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;

use chrono::{TimeDelta, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use test_context::AsyncTestContext;
use url::Url;
use uuid::Uuid;

/// Credentials of the e2e user for real tenants; unset means the emulator issuer.
const TEST_USER: &str = "TOPK_TEST_USER";
const TEST_PASSWORD: &str = "TOPK_TEST_PASSWORD";
/// Auth0 database connection the e2e user belongs to.
const REALM: &str = "Username-Password-Authentication";
/// Reuse a minted session while it has at least this long to live.
const SESSION_MIN_REMAINING: TimeDelta = TimeDelta::minutes(15);
/// Projects this crate creates; older leftovers are swept at setup.
const PROJECT_PREFIX: &str = "it-";
const STALE_AFTER: TimeDelta = TimeDelta::days(1);

/// A config directory of its own, logged in as `topk login` would leave it, so
/// tests never touch the developer's sessions.
pub struct CliTestContext {
    /// Removed with the context; the process env points at it (see `logged_out`).
    _home: TempDir,
    projects: Mutex<HashSet<String>>,
}

impl CliTestContext {
    pub fn logged_out() -> Self {
        env("TOPK_AUTH_ISSUER");
        let home = tempfile::tempdir().unwrap();
        // Point this process at the temporary home too, so `topk::config::dir()`
        // resolves the same directory the spawned CLI will use. Sound under
        // nextest, which runs every test in a process of its own.
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_CONFIG_HOME", home.path());
        CliTestContext {
            _home: home,
            projects: Mutex::new(HashSet::new()),
        }
    }

    pub async fn login(&self) {
        match (var(TEST_USER), var(TEST_PASSWORD)) {
            (Some(user), Some(password)) => self.password_grant(&user, &password).await,
            _ => self.browserless_login().await,
        }
    }

    /// `topk login --no-browser` against the emulator issuer, which redirects
    /// straight back to the callback with no login page in between.
    async fn browserless_login(&self) {
        let mut child = self
            .command(&["login", "--no-browser"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn topk login");
        // Keep reading until exit: the CLI dies of SIGPIPE if stderr closes under it.
        let mut stderr = BufReader::new(child.stderr.take().unwrap())
            .lines()
            .map(Result::unwrap);
        let url = stderr
            .by_ref()
            .find(|line| line.starts_with("http"))
            .expect("login url");
        let page = reqwest::get(url).await.expect("follow login url");
        assert!(
            page.status().is_success(),
            "callback status {}",
            page.status()
        );
        assert!(page.text().await.unwrap().contains("You're logged in"));
        let rest: Vec<String> = stderr.collect();
        assert!(
            child.wait().unwrap().success(),
            "topk login failed:\n{}",
            rest.join("\n")
        );
        assert!(
            rest.iter().any(|line| line.contains("Logged in")),
            "{rest:?}"
        );
    }

    /// A real tenant has no scriptable login page: mint the session with the
    /// password grant of the tenant's e2e CLI application and store it exactly
    /// where and how `topk login` does, so every command runs as after a login.
    ///
    /// One grant per machine while the session is fresh, not one per test:
    /// nextest runs each test in its own process, and the tenant counts every
    /// rejected grant as a failed login, which can block the e2e user.
    async fn password_grant(&self, user: &str, password: &str) {
        let issuer = Url::parse(&env("TOPK_AUTH_ISSUER")).expect("issuer url");
        let client_id = env("TOPK_AUTH_CLIENT_ID");
        let audience = env("TOPK_AUTH_AUDIENCE");
        let cache_dir = std::env::temp_dir().join(format!(
            "topk-management-tests-{:x}",
            Sha256::digest(format!("{issuer}{client_id}{audience}{user}"))
        ));
        fs::create_dir_all(&cache_dir).unwrap();
        let lock = File::create(cache_dir.join("lock")).unwrap();
        lock.lock().unwrap();
        let cached = cache_dir.join("credentials.toml");
        if !session_is_fresh(&cached) {
            let response = reqwest::Client::new()
                .post(issuer.join("oauth/token").unwrap())
                .form(&[
                    (
                        "grant_type",
                        "http://auth0.com/oauth/grant-type/password-realm",
                    ),
                    ("realm", REALM),
                    ("username", user),
                    ("password", password),
                    ("client_id", &client_id),
                    ("audience", &audience),
                    ("scope", "openid profile email offline_access"),
                ])
                .send()
                .await
                .expect("token request");
            let status = response.status();
            let token: Value = response.json().await.expect("token response");
            assert!(status.is_success(), "password grant failed: {token}");
            let credentials = json!({
                "client_id": client_id,
                "audience": audience,
                "access_token": token["access_token"].as_str().expect("access token"),
                "refresh_token": token["refresh_token"].as_str().expect("refresh token"),
                "expires_at": Utc::now().timestamp() + token["expires_in"].as_i64().expect("expires_in"),
            });
            write_secret(&cached, &toml::to_string(&credentials).unwrap());
        }
        let session = self
            .config_dir()
            .join("tenants")
            .join(format!("{:x}", Sha256::digest(issuer.as_str().as_bytes())))
            .join("credentials.toml");
        write_secret(&session, &fs::read_to_string(&cached).unwrap());
        lock.unlock().unwrap();
    }

    fn config_dir(&self) -> PathBuf {
        topk::config::dir().expect("config directory")
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_topk"));
        cmd.args(args)
            .env_remove("TOPK_API_KEY")
            .env("TOPK_AUTH_CALLBACK_PORTS", "0");
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
    pub fn create_project(&self, name: &str) -> Value {
        let project = self
            .json(&["project", "create", name])
            .pop()
            .expect("created project");
        self.projects
            .lock()
            .unwrap()
            .insert(project["project_id"].as_str().unwrap().to_string());
        project
    }

    /// Like the sandbox does for collections: remove what an aborted earlier
    /// run left behind, old enough not to belong to a test running right now.
    fn sweep_stale_projects(&self) {
        let cutoff = (Utc::now() - STALE_AFTER).timestamp();
        for project in self.json(&["project", "list"]) {
            let stale = project["name"]
                .as_str()
                .unwrap()
                .starts_with(PROJECT_PREFIX)
                && project["created_at"].as_i64().unwrap() < cutoff;
            if stale {
                let id = project["project_id"].as_str().unwrap();
                println!("Deleting stale project {id} ({})", project["name"]);
                self.run(&["project", "delete", "--yes", id]);
            }
        }
    }
}

impl AsyncTestContext for CliTestContext {
    async fn setup() -> Self {
        let ctx = CliTestContext::logged_out();
        ctx.login().await;
        ctx.sweep_stale_projects();
        ctx
    }

    async fn teardown(self) {
        for id in std::mem::take(&mut *self.projects.lock().unwrap()) {
            let out = self.run(&["project", "delete", "--yes", &id]);
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

fn session_is_fresh(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| raw.parse::<toml::Table>().ok())
        .and_then(|table| table.get("expires_at").and_then(toml::Value::as_integer))
        .is_some_and(|expires_at| expires_at > (Utc::now() + SESSION_MIN_REMAINING).timestamp())
}

fn write_secret(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn env(name: &str) -> String {
    var(name)
        .unwrap_or_else(|| panic!("{name} not set: management tests need an issuer to log in to"))
}
