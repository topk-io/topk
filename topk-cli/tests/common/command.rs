// Shared by several test crates; each uses a different part of it.
#![allow(dead_code)]

use std::borrow::Borrow;
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Runs the `topk` binary under test, as a user would from a shell.
pub struct TestCommand {
    command: Command,
    stdin: Option<String>,
}

impl TestCommand {
    pub fn new() -> Self {
        Self {
            command: Command::new(env!("CARGO_BIN_EXE_topk")),
            stdin: None,
        }
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.command.args(args);
        self
    }

    pub fn envs<K: AsRef<OsStr>, V: AsRef<OsStr>>(
        mut self,
        envs: impl IntoIterator<Item = impl Borrow<(K, V)>>,
    ) -> Self {
        for env in envs {
            let (key, value) = env.borrow();
            self.command.env(key, value);
        }
        self
    }

    pub fn stdin(mut self, input: Option<&str>) -> Self {
        self.stdin = input.map(str::to_owned);
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

    /// Stdout and stderr of a run that must succeed.
    pub fn succeeds(self) -> (String, String) {
        let line = self.line();
        let output = self.output();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "`{line}` failed:\n{stderr}");
        (String::from_utf8(output.stdout).unwrap(), stderr)
    }

    /// Stdout of a run that must succeed.
    pub fn ok(self) -> String {
        self.succeeds().0
    }

    /// Stdout of a run that must succeed, one JSON value per line.
    pub fn json(self) -> Vec<Value> {
        self.ok()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
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
