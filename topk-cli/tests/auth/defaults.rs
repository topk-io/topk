use clap::{Args, Command, FromArgMatches};
use tempfile::TempDir;

use topk::auth::OAuthConfig;
use topk::commands::project;
use topk::config::Config;
use topk::endpoint::{Credentials, DataEndpoint};
use topk::host::Host;
use topk::output::Output;

use crate::common::host;

fn config(dir: &TempDir) -> Config {
    let matches = OAuthConfig::augment_args(Command::new("test"))
        .mut_args(|arg| arg.env(None::<&str>))
        .get_matches_from(["test"]);
    Config::new(
        host(),
        OAuthConfig::from_arg_matches(&matches).unwrap(),
        dir.path().to_owned(),
    )
}

#[tokio::test]
async fn defaults_persist_and_are_scoped_to_host_and_issuer() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    assert!(config.defaults().unwrap().project_id.is_none());
    config.set_project_id(Some("p1".parse().unwrap())).unwrap();
    let loaded = Config::new(host(), config.oauth().clone(), dir.path().to_owned());
    assert_eq!(
        loaded.defaults().unwrap().project_id.unwrap().as_str(),
        "p1"
    );

    let other = Host {
        host: "other.test".into(),
        https: true,
    };
    assert!(
        Config::new(other, config.oauth().clone(), dir.path().to_owned())
            .defaults()
            .unwrap()
            .project_id
            .is_none()
    );
    let mut oauth = config.oauth().clone();
    oauth.issuer = "https://other.test/".parse().unwrap();
    assert!(Config::new(host(), oauth, dir.path().to_owned())
        .defaults()
        .unwrap()
        .project_id
        .is_none());

    config.session().await.unwrap().delete().unwrap();
    assert_eq!(
        config.defaults().unwrap().project_id.unwrap().as_str(),
        "p1"
    );
}

#[tokio::test]
async fn current_and_clear_work_offline() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    config.set_project_id(Some("p1".parse().unwrap())).unwrap();
    let mut out = Vec::new();
    project::run(
        config.clone(),
        &project::Args {
            command: project::Command::Current,
        },
        Output::Json,
        &mut out,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["project_id"],
        "p1"
    );
    project::run(
        config.clone(),
        &project::Args {
            command: project::Command::Clear,
        },
        Output::Text,
        &mut Vec::new(),
    )
    .await
    .unwrap();
    assert!(config.defaults().unwrap().project_id.is_none());
    let error = project::run(
        config.clone(),
        &project::Args {
            command: project::Command::Current,
        },
        Output::Json,
        &mut Vec::new(),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("no project selected"), "{error}");
}

#[test]
fn credentials_fall_back_to_the_selected_project() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    let endpoint = |api_key: Option<&str>, project_id: Option<&str>| DataEndpoint {
        api_key: api_key.map(Into::into),
        project_id: project_id.map(|id| id.parse().unwrap()),
        region: None,
    };
    let project = |endpoint: DataEndpoint| match endpoint.credentials(&config).unwrap() {
        Credentials::Project(id) => id.as_str().to_owned(),
        Credentials::ApiKey(_) => panic!("expected project credentials"),
    };

    let error = endpoint(None, None)
        .credentials(&config)
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("topk project select"), "{error}");

    config
        .set_project_id(Some("selected".parse().unwrap()))
        .unwrap();
    assert_eq!(project(endpoint(None, None)), "selected");
    assert_eq!(project(endpoint(None, Some("flag"))), "flag");
    assert!(matches!(
        endpoint(Some("key"), None).credentials(&config).unwrap(),
        Credentials::ApiKey(key) if key == "key"
    ));
}
