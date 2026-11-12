use clap::{Args, Command, FromArgMatches};
use rstest::rstest;
use tempfile::TempDir;

use topk::auth::OAuthConfig;
use topk::commands::{project, region};
use topk::config::Config;
use topk::endpoint::{ProjectArg, RegionArg};
use topk::host::Host;
use topk::output::Output;

use super::common::host;

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
        .update_defaults(|defaults| {
            defaults.project_id = Some("p1".parse().unwrap());
            defaults.region = Some("r1".into());
        })
        .unwrap();
    let loaded = Config::new(host(), config.oauth().clone(), dir.path().to_owned());
    assert_eq!(
        loaded.defaults().unwrap().project_id.unwrap().as_str(),
        "p1"
    );
    assert_eq!(loaded.defaults().unwrap().region.unwrap().as_str(), "r1");

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
    config
        .update_defaults(|defaults| defaults.project_id = Some("p1".parse().unwrap()))
        .unwrap();
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

#[tokio::test]
async fn region_current_and_clear_work_offline() {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    config
        .update_defaults(|defaults| {
            defaults.project_id = Some("p1".parse().unwrap());
            defaults.region = Some("r1".into());
        })
        .unwrap();
    let run = |command| {
        let config = config.clone();
        async move {
            let mut out = Vec::new();
            region::run(config, &region::Args { command }, Output::Json, &mut out)
                .await
                .map(|_| out)
        }
    };
    let out = run(region::Command::Current).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["region"],
        "r1"
    );
    run(region::Command::Clear).await.unwrap();
    let defaults = config.defaults().unwrap();
    assert!(defaults.region.is_none());
    assert_eq!(defaults.project_id.unwrap().as_str(), "p1");
    let error = run(region::Command::Current).await.unwrap_err();
    assert!(error.to_string().contains("no region selected"), "{error}");
}

#[rstest]
#[case::flag(&["test", "--project-id", "flag", "--region", "flag"], Some("TOPK_TEST_FLAG"), "flag")]
#[case::environment(&["test"], Some("TOPK_TEST_ENVIRONMENT"), "environment")]
#[case::selected_default(&["test"], None, "saved")]
#[tokio::test]
async fn project_and_region_resolve_in_order(
    #[case] args: &[&str],
    #[case] env: Option<&'static str>,
    #[case] expected: &str,
) {
    let dir = TempDir::new().unwrap();
    let config = config(&dir);
    config
        .update_defaults(|defaults| {
            defaults.project_id = Some("saved".parse().unwrap());
            defaults.region = Some("saved".into());
        })
        .unwrap();
    // Each case reads its own variable, so cases cannot affect each other.
    if let Some(env) = env {
        std::env::set_var(env, "environment");
    }
    let matches = RegionArg::augment_args(ProjectArg::augment_args(Command::new("test")))
        .mut_args(|arg| arg.env(env))
        .get_matches_from(args);
    let project = ProjectArg::from_arg_matches(&matches)
        .unwrap()
        .resolve(&config)
        .await
        .unwrap();
    let region = RegionArg::from_arg_matches(&matches)
        .unwrap()
        .resolve(&config)
        .await
        .unwrap();
    assert_eq!((project.as_str(), region.as_str()), (expected, expected));
}
