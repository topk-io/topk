use serde_json::Value;
use std::collections::BTreeMap;
use topk::import::spec::Spec;
use topk::import::Outcome;

use crate::common::command::CliOutput;

impl CliOutput {
    #[track_caller]
    pub fn spec(&self) -> Spec {
        toml::from_str(&self.stdout).unwrap_or_else(|error| panic!("{error}\n{self:?}"))
    }

    #[track_caller]
    pub fn summary(&self) -> BTreeMap<String, Outcome> {
        serde_json::from_str(&self.stdout).unwrap_or_else(|error| panic!("{error}\n{self:?}"))
    }

    #[track_caller]
    pub fn id(&self) -> &str {
        self.stderr
            .lines()
            .find_map(|line| line.split_once("--resume "))
            .and_then(|(_, rest)| rest.split_whitespace().next())
            .unwrap_or_else(|| panic!("missing resume command\n{self:?}"))
    }

    #[track_caller]
    pub fn preview(&self) -> Vec<Value> {
        self.stderr
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| {
                serde_json::from_str(line).unwrap_or_else(|error| panic!("{error}\n{self:?}"))
            })
            .collect()
    }
}
