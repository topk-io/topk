#![cfg(unix)]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rstest::rstest;

use topk::pager::Pager;

// Run in a separate process so pager environment and SIGPIPE changes cannot affect other tests.
#[test]
#[ignore]
fn pager_child() {
    // Match the CLI's default signal handling before a pager is spawned.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    let rows: usize = std::env::var("PAGER_TEST_ROWS").unwrap().parse().unwrap();
    let mut pager = Pager::stdout();
    let result = (|| -> anyhow::Result<()> {
        for _ in 0..rows {
            writeln!(pager, "result-row")?;
        }
        pager.flush()?;
        Ok(())
    })();
    pager.finish().unwrap();
    if !result.as_ref().is_err_and(Pager::quit) {
        result.unwrap();
    }
}

fn run(
    rows: usize,
    terminal: bool,
    topk_pager: Option<&str>,
    pager: Option<&str>,
    no_path: bool,
) -> (bool, String) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", "pager_child", "--nocapture"])
        .env("PAGER_TEST_ROWS", rows.to_string())
        .env_remove("TOPK_PAGER")
        .env_remove("PAGER")
        .env_remove("LESS")
        .stdin(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(value) = topk_pager {
        command.env("TOPK_PAGER", value);
    }
    if let Some(value) = pager {
        command.env("PAGER", value);
    }
    if no_path {
        command.env("PATH", "");
    }

    let reader = if terminal {
        let mut master = -1;
        let mut slave = -1;
        let mut size = libc::winsize {
            ws_row: 10,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // openpty initializes two owned descriptors on success. Only the child owns the slave.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut size,
                )
            },
            0
        );
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        command.stdout(slave);
        Some(thread::spawn(move || {
            let mut master = master;
            let mut output = Vec::new();
            let mut buf = [0; 4096];
            loop {
                match master.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => output.extend_from_slice(&buf[..n]),
                    // Linux reports EIO when the slave closes; macOS returns EOF.
                    Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                    Err(error) => panic!("reading terminal: {error}"),
                }
            }
            output
        }))
    } else {
        command.stdout(Stdio::piped());
        None
    };
    let mut child = command.spawn().unwrap();
    drop(command);
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("pager child did not exit");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    let stdout = reader
        .map(|reader| reader.join().unwrap())
        .unwrap_or(output.stdout);
    (output.status.success(), String::from_utf8(stdout).unwrap())
}

#[rstest]
#[case::short_output(2, true, Some("printf 'pager-started\\n'; cat"), None, false, false)]
#[case::long_output(100, true, Some("printf 'pager-started\\n'; cat"), None, false, true)]
#[case::redirected_output(100, false, Some("printf 'pager-started\\n'; cat"), None, false, false)]
#[case::empty_topk_pager(
    100,
    true,
    Some(""),
    Some("printf 'pager-started\\n'; cat"),
    false,
    false
)]
#[case::empty_pager(100, true, None, Some(""), false, false)]
#[case::cat(100, true, Some("cat"), None, false, false)]
#[case::pager_fallback(100, true, None, Some("printf 'pager-started\\n'; cat"), false, true)]
#[case::topk_precedence(
    100,
    true,
    Some("printf 'pager-started\\n'; cat"),
    Some("exit 99"),
    false,
    true
)]
#[case::missing_less(100, true, None, None, true, false)]
fn routes_output(
    #[case] rows: usize,
    #[case] terminal: bool,
    #[case] topk_pager: Option<&str>,
    #[case] pager: Option<&str>,
    #[case] no_path: bool,
    #[case] paged: bool,
) {
    let (success, output) = run(rows, terminal, topk_pager, pager, no_path);
    assert!(success, "{output}");
    assert_eq!(output.contains("pager-started"), paged, "{output}");
    assert_eq!(output.matches("result-row").count(), rows, "{output}");
}

#[test]
fn quitting_early_does_not_fail_or_hang() {
    let (success, output) = run(100_000, true, Some("head -n 1"), None, false);
    assert!(success, "{output}");
    assert_eq!(output.matches("result-row").count(), 1);
}

#[test]
fn invalid_configured_pager_is_an_error() {
    let (success, _) = run(100, true, Some("exec /topk-missing-pager"), None, false);
    assert!(!success);
}

#[test]
fn database_broken_pipe_is_not_a_pager_exit() {
    let error = anyhow::Error::new(sqlx::Error::Io(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "database connection closed",
    )))
    .context("executing SQL");
    assert!(!Pager::quit(&error));
}

#[test]
fn unrelated_output_broken_pipe_is_not_a_pager_exit() {
    let error = anyhow::Error::new(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "stdout closed",
    ));
    assert!(!Pager::quit(&error));
}
