use clap::{Args, Command, FromArgMatches};
use tempfile::TempDir;

use topk::auth::OAuthConfig;
use topk::commands::project;
use topk::config::Config;
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
    config
        .set_project_id(Some("p1".parse().unwrap()))
        .await
        .unwrap();
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
async fn current_and_unselect_work_offline() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    config
        .set_project_id(Some("p1".parse().unwrap()))
        .await
        .unwrap();
    let mut out = Vec::new();
    project::run(
        config.clone(),
        &project::Args { command: None },
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
            command: Some(project::Command::Unselect),
        },
        Output::Text,
        &mut Vec::new(),
    )
    .await
    .unwrap();
    assert!(config.defaults().unwrap().project_id.is_none());
}
