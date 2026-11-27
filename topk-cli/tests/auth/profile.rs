use rstest::rstest;
use serde_json::{json, Value};
use tempfile::TempDir;

use topk::config::Config;
use topk::host::Host;
use topk::profile::SavedProfiles;

use super::common::args;
use crate::common::command::TestCommand;

const AUTH_ISSUER: &str = "https://auth.example.test/";

fn profiles(dir: &TempDir, names: &[&str]) -> SavedProfiles {
    let profiles = SavedProfiles::new(dir.path().to_owned());
    for profile in names {
        let host = Host {
            host: "stage.test".into(),
            https: true,
        };
        profiles.save(&profile.parse().unwrap(), &host).unwrap();
    }
    profiles
}

/// The config of a profile logged in to `host`.
async fn config(dir: &TempDir, profile: &str, host: &str, https: bool) -> Config {
    Config::load(
        &args(&[
            "test",
            "--profile",
            profile,
            "--host",
            host,
            "--https",
            &https.to_string(),
            "--auth-issuer",
            AUTH_ISSUER,
            "--auth-client-id",
            "cli",
            "--auth-audience",
            "https://api.example.test",
        ]),
        SavedProfiles::new(dir.path().to_owned()),
    )
    .await
    .unwrap()
}

fn topk(dir: &TempDir) -> TestCommand {
    TestCommand::new()
        .env_remove("TOPK_PROFILE")
        .env_remove("TOPK_HOST")
        .env_remove("TOPK_HTTPS")
        .env_remove("TOPK_REGION")
        .env_remove("TOPK_PROJECT_ID")
        .envs([
            ("TOPK_CONFIG_DIR", dir.path().to_str().unwrap()),
            ("TOPK_AUTH_ISSUER", AUTH_ISSUER),
            ("TOPK_AUTH_CLIENT_ID", "cli"),
            ("TOPK_AUTH_AUDIENCE", "https://api.example.test"),
        ])
}

fn current(command: TestCommand) -> Value {
    command.args(["profile", "current", "-o", "json"]).json()[0]["profile"].clone()
}

#[rstest]
#[case::default(None, None, None, "default")]
#[case::selected(Some("one"), None, None, "one")]
#[case::env_over_selected(Some("one"), Some("two"), None, "two")]
#[case::flag_over_env(Some("one"), Some("two"), Some("default"), "default")]
fn profile_precedence(
    #[case] selected: Option<&str>,
    #[case] env: Option<&str>,
    #[case] flag: Option<&str>,
    #[case] expected: &str,
) {
    let dir = TempDir::new().unwrap();
    let profiles = profiles(&dir, &["one", "two"]);
    if let Some(selected) = selected {
        profiles.select(&selected.parse().unwrap()).unwrap();
    }
    let mut command = topk(&dir);
    if let Some(env) = env {
        command = command.envs([("TOPK_PROFILE", env)]);
    }
    if let Some(flag) = flag {
        command = command.args(["--profile", flag]);
    }
    assert_eq!(current(command), expected);
}

#[test]
fn select_list_and_clear() {
    let dir = TempDir::new().unwrap();
    profiles(&dir, &["stage"]);

    assert_eq!(
        topk(&dir)
            .args(["profile", "select", "stage", "-o", "json"])
            .json(),
        [json!({"profile": "stage"})]
    );
    assert_eq!(
        topk(&dir).args(["profile", "list", "-o", "json"]).json(),
        [json!({"name": "stage", "host": "stage.test", "https": true, "selected": true})]
    );
    let table = topk(&dir).args(["profile", "list"]).ok();
    assert!(table.contains("stage*"), "{table}");

    topk(&dir).args(["profile", "clear"]).ok();
    assert_eq!(current(topk(&dir)), "default");
}

#[rstest]
#[case::select(&["profile", "select", "nope"], "profile `nope` not found")]
#[case::invalid_name(&["--profile", "No/pe", "profile", "current"], "invalid profile name")]
#[case::no_subcommand(&["profile"], "Usage: topk profile")]
fn profile_errors(#[case] args: &[&str], #[case] error: &str) {
    let dir = TempDir::new().unwrap();
    let stderr = topk(&dir).args(args).fails();
    assert!(stderr.contains(error), "{stderr}");
}

#[tokio::test]
async fn profile_keeps_its_own_defaults_and_host() {
    let dir = TempDir::new().unwrap();
    profiles(&dir, &["one", "two"]);
    // Same host and issuer, so only the profile tells the two apart.
    for (profile, region) in [("one", "r1"), ("two", "r2")] {
        config(&dir, profile, "stage.test", true)
            .await
            .set_region(Some(region.into()))
            .unwrap();
    }
    let region = |args: &[&str]| topk(&dir).args(args).args(["region", "current"]);
    assert_eq!(region(&["--profile", "one"]).ok(), "Default region: r1\n");
    assert_eq!(region(&["--profile", "two"]).ok(), "Default region: r2\n");
    // Another profile and an explicit host both look elsewhere.
    assert!(region(&[]).fails().contains("no region selected"));
    assert!(region(&["--profile", "one", "--host", "other.test"])
        .fails()
        .contains("no region selected"));
}

#[test]
fn logout_removes_the_profile() {
    let dir = TempDir::new().unwrap();
    let profiles = profiles(&dir, &["stage"]);
    profiles.select(&"stage".parse().unwrap()).unwrap();

    topk(&dir).args(["logout"]).ok();

    assert!(profiles.load().unwrap().list().next().is_none());
    assert_eq!(current(topk(&dir)), "default");
    assert!(topk(&dir)
        .args(["profile", "select", "stage"])
        .fails()
        .contains("profile `stage` not found"));
}

#[tokio::test]
async fn host_given_does_not_inherit_the_profile_scheme() {
    let dir = TempDir::new().unwrap();
    let plain = Host {
        host: "plain.test".into(),
        https: false,
    };
    SavedProfiles::new(dir.path().to_owned())
        .save(&"plain".parse().unwrap(), &plain)
        .unwrap();
    config(&dir, "plain", "other.test", true)
        .await
        .set_region(Some("r1".into()))
        .unwrap();
    assert_eq!(
        topk(&dir)
            .args(["--profile", "plain", "--host", "other.test"])
            .args(["region", "current"])
            .ok(),
        "Default region: r1\n"
    );
}

#[tokio::test]
async fn https_given_applies_to_the_profile_host() {
    let dir = TempDir::new().unwrap();
    profiles(&dir, &["stage"]);
    config(&dir, "stage", "stage.test", false)
        .await
        .set_region(Some("r1".into()))
        .unwrap();
    let region = |args: &[&str]| {
        topk(&dir)
            .args(["--profile", "stage"])
            .args(args)
            .args(["region", "current"])
    };
    assert_eq!(region(&["--https", "false"]).ok(), "Default region: r1\n");
    assert!(region(&[]).fails().contains("no region selected"));
}

#[test]
fn logging_in_again_does_not_restore_an_old_selection() {
    let dir = TempDir::new().unwrap();
    let profiles = profiles(&dir, &["stage"]);
    profiles.select(&"stage".parse().unwrap()).unwrap();
    topk(&dir).args(["logout"]).ok();

    self::profiles(&dir, &["stage"]);

    assert_eq!(current(topk(&dir)), "default");
}

#[test]
fn invalid_saved_profile_name_is_rejected() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("profiles.toml"),
        "[profiles.\"No/pe\"]\nhost = \"stage.test\"\nhttps = true\n",
    )
    .unwrap();
    let stderr = topk(&dir).args(["profile", "list"]).fails();
    assert!(stderr.contains("invalid profile name"), "{stderr}");
}

#[test]
fn legacy_config_is_removed() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "api_key = 'legacy-secret'\n").unwrap();
    topk(&dir).args(["profile", "current"]).ok();
    assert!(!path.exists());
}
