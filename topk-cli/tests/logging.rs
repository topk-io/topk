use std::process::Command;

use rstest::rstest;
use tempfile::TempDir;

#[rstest]
#[case::quiet(&[], None, false, false)]
#[case::info(&["-v"], None, true, false)]
#[case::debug(&["-vv"], None, true, true)]
#[case::extra_verbose(&["-vvv"], None, true, true)]
#[case::environment_override(&["-vv"], Some("warn"), false, false)]
fn verbosity_levels(
    #[case] flags: &[&str],
    #[case] filter: Option<&str>,
    #[case] info: bool,
    #[case] debug: bool,
) {
    let dir = TempDir::new().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_topk"));
    command.env_clear().env("TOPK_CONFIG_DIR", dir.path());
    if let Some(filter) = filter {
        command.env("RUST_LOG", filter);
    }
    let output = command
        .args(flags)
        .args(["sql", "SELECT 'secret-marker'", "--project-id", "p1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.contains(" INFO "), info, "{stderr}");
    assert_eq!(stderr.contains(" DEBUG "), debug, "{stderr}");
    assert!(stderr.contains("--region"), "{stderr}");
}

#[test]
fn verbose_json_stays_parseable() {
    let dir = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_topk"))
        .env_clear()
        .env("TOPK_CONFIG_DIR", dir.path())
        .args(["-vv", "-o", "json", "region", "clear"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value, serde_json::json!({"region": null}));
    assert!(String::from_utf8(output.stderr).unwrap().contains(" INFO "));
}

#[cfg(feature = "import")]
#[test]
fn import_logs_hide_the_source_password() {
    let dir = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_topk"))
        .env_clear()
        .env("TOPK_CONFIG_DIR", dir.path())
        .args([
            "-vv",
            "import",
            "postgres://u:secret-marker@127.0.0.1:1/db",
            "--to",
            "c",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("u:***@"), "{stderr}");
    assert!(!stderr.contains("secret-marker"), "{stderr}");
}

#[cfg(unix)]
#[rstest]
#[case::json_output(&["-o", "json", "region", "clear"])]
#[case::completions(&["completions", "bash"])]
fn closed_stdout_exits_quietly(#[case] args: &[&str]) {
    let dir = TempDir::new().unwrap();
    let (stdout, reader) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_topk"))
        .env_clear()
        .env("TOPK_CONFIG_DIR", dir.path())
        .args(args)
        .stdout(std::os::fd::OwnedFd::from(stdout))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(141), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}
