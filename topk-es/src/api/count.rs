use serde::{Deserialize, Serialize};

use super::query::GateQuery;
use super::Shards;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountRequest {
    #[serde(default)]
    pub query: Option<GateQuery>,
}

#[derive(Serialize)]
pub struct CountBody {
    pub count: u64,
    #[serde(rename = "_shards")]
    pub shards: Shards,
}

impl CountBody {
    pub fn new(count: u64) -> Self {
        Self {
            count,
            shards: Shards::default(),
        }
    }
}
