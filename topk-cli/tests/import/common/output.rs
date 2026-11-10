use std::collections::BTreeMap;
use std::fmt;
use std::process::ExitStatus;

use serde_json::Value;
use topk::import::Spec;

use crate::import::common::TestCommand;

/// `topk!("import", "-f", spec, "--limit", 5)`: arguments of any `Display` type.
/// `topk!(command; "-f", spec)` extends a prepared command, for a call that
/// needs its own environment.
macro_rules! topk {
    ($command:expr; $($arg:expr),+ $(,)?) => {
        $crate::import::common::CliOutput::of($command.args(["-o", "json"]).args([$($arg.to_string()),+]))
    };
    ($($arg:expr),+ $(,)?) => {
        topk!($crate::import::common::topk(); $($arg),+)
    };
}

/// One finished `topk … -o json` call, parsed.
pub struct CliOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutput {
    /// `Err` when the command exits non-zero; a failed import still has its counts.
    pub fn of(command: TestCommand) -> Result<CliOutput, CliOutput> {
        let run = CliOutput::from(command.output());
        match run.status.success() {
            true => Ok(run),
            false => Err(run),
        }
    }

    /// The spec a dry run prints.
    #[track_caller]
    pub fn spec(&self) -> Spec {
        toml::from_str(&self.stdout).expect("printed spec parses")
    }

    pub fn summary(&self) -> BTreeMap<String, Value> {
        serde_json::from_str(&self.stdout).expect("import summary")
    }

    pub fn rows(&self, collection: &str) -> u64 {
        self.summary()[collection]["rows"].as_u64().expect("rows")
    }

    pub fn failed(&self, collection: &str) -> u64 {
        self.summary()[collection]["failed"]
            .as_u64()
            .expect("failed")
    }

    pub fn id(&self) -> &str {
        self.stderr
            .lines()
            .find_map(|line| line.strip_prefix("# run "))
            .and_then(|rest| rest.split(',').next())
            .expect("run header")
    }

    pub fn docs(&self) -> Vec<Value> {
        self.stderr
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| serde_json::from_str(line).expect("preview document"))
            .collect()
    }
}

impl fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\n{}{}", self.stdout, self.stderr)
    }
}

impl From<std::process::Output> for CliOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            status: output.status,
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }
}
