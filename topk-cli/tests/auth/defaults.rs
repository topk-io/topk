use clap::{Args, Command, FromArgMatches};
use tempfile::TempDir;

use topk::auth::OAuthConfig;
use topk::commands::{project, region};
use topk::config::{Config, Defaults};
use topk::host::Host;
use topk::output::Output;
use topk::Region;

fn config(dir: &TempDir) -> Config {
    let matches = OAuthConfig::augment_args(Host::augment_args(Command::new("test")))
        .mut_args(|arg| arg.env(None::<&str>))
        .get_matches_from(["test"]);
    Config::new(
        Host::from_arg_matches(&matches).unwrap(),
        OAuthConfig::from_arg_matches(&matches).unwrap(),
        dir.path().to_owned(),
    )
}

#[test]
fn saved_regions_accept_existing_strings_and_reject_empty_values() {
    let defaults: Defaults = toml::from_str("region = 'local-0-emulator'").unwrap();
    assert_eq!(defaults.region.unwrap().as_str(), "local-0-emulator");
    assert!(toml::from_str::<Defaults>("region = ''").is_err());
}

#[tokio::test]
async fn defaults_persist_independently_and_are_scoped_to_host_and_issuer() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    assert!(config.defaults().unwrap().project_id.is_none());
    let (project, region) = tokio::join!(
        config.set_project_id(Some("p1".parse().unwrap())),
        config.set_region(Some("region-1".parse().unwrap())),
    );
    project.unwrap();
    region.unwrap();
    let loaded = Config::new(
        config.host().clone(),
        config.oauth().clone(),
        dir.path().to_owned(),
    );
    assert_eq!(
        loaded.defaults().unwrap().project_id.unwrap().as_str(),
        "p1"
    );
    assert_eq!(
        loaded
            .defaults()
            .unwrap()
            .region
            .as_ref()
            .map(Region::as_str),
        Some("region-1")
    );

    let mut host = config.host().clone();
    host.host = "other.test".into();
    assert!(
        Config::new(host, config.oauth().clone(), dir.path().to_owned())
            .defaults()
            .unwrap()
            .project_id
            .is_none()
    );
    let mut oauth = config.oauth().clone();
    oauth.issuer = "https://other.test/".parse().unwrap();
    assert!(
        Config::new(config.host().clone(), oauth, dir.path().to_owned())
            .defaults()
            .unwrap()
            .region
            .is_none()
    );

    config.session().await.unwrap().delete().unwrap();
    assert_eq!(
        config
            .defaults()
            .unwrap()
            .region
            .as_ref()
            .map(Region::as_str),
        Some("region-1")
    );
}

#[tokio::test]
async fn current_and_unselect_work_offline_and_preserve_the_other_selection() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    config
        .set_project_id(Some("p1".parse().unwrap()))
        .await
        .unwrap();
    config
        .set_region(Some("region-1".parse().unwrap()))
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
    out.clear();
    region::run(
        config.clone(),
        &region::Args { command: None },
        Output::Json,
        &mut out,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["region"],
        "region-1"
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
    assert_eq!(
        config
            .defaults()
            .unwrap()
            .region
            .as_ref()
            .map(Region::as_str),
        Some("region-1")
    );
    region::run(
        config.clone(),
        &region::Args {
            command: Some(region::Command::Unselect),
        },
        Output::Text,
        &mut Vec::new(),
    )
    .await
    .unwrap();
    assert!(config.defaults().unwrap().region.is_none());
}
