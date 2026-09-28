// Shared by several test crates; each uses a different part of it.
#![allow(dead_code)]

use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Runs the `topk` binary under test, as a user would from a shell.
pub struct TestCommand {
    command: Command,
    stdin: Option<String>,
}

impl TestCommand {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_topk"));
        command.args(args);
        Self {
            command,
            stdin: None,
        }
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.command.env(key, value);
        self
    }

    pub fn stdin(mut self, input: &str) -> Self {
        self.stdin = Some(input.to_owned());
        self
    }

    pub fn output(mut self) -> Output {
        let mut child = self
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run topk");
        // Closing stdin right away lets a command that reads it see the end.
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(self.stdin.unwrap_or_default().as_bytes())
            .unwrap();
        drop(stdin);
        child.wait_with_output().unwrap()
    }

    /// Stdout of a run that must succeed.
    pub fn ok(self) -> String {
        let line = self.line();
        let output = self.output();
        assert!(
            output.status.success(),
            "`{line}` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// Stderr of a run that must fail.
    pub fn fails(self) -> String {
        let line = self.line();
        let output = self.output();
        assert!(!output.status.success(), "`{line}` should have failed");
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn line(&self) -> String {
        std::iter::once("topk".into())
            .chain(self.command.get_args().map(|arg| arg.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
