use serde::Serialize;

mod aggs;
mod body;
mod bulk;
mod count;
mod doc;
mod index;
mod mapping;
mod mget;
mod msearch;
mod ndjson;
pub mod alias;
mod alias_api;
pub use alias_api::*;
mod pit;
pub use pit::*;
mod path;
mod query;
mod refresh;
mod search;
mod source;
mod unavailable;
mod write;

pub use aggs::*;
pub use body::*;
pub use bulk::*;
pub use count::*;
pub use doc::*;
pub use index::*;
pub use mapping::*;
pub use mget::*;
pub use msearch::*;
pub use path::*;
pub use query::*;
pub use refresh::*;
pub use search::*;
pub use source::*;
pub use unavailable::*;
pub use write::*;

#[derive(Clone, Serialize)]
pub struct Shards {
    pub total: u32,
    pub successful: u32,
    pub failed: u32,
}

#[derive(Clone, Serialize)]
pub struct RefreshBody {
    #[serde(rename = "_shards")]
    pub shards: Shards,
}

// STUB: reports that nothing matched. Correct on an empty index, a lie on a populated one.
#[derive(Serialize)]
pub struct UpdateByQueryBody {
    pub took: u32,
    pub timed_out: bool,
    pub total: u64,
    pub updated: u64,
    pub deleted: u64,
    pub batches: u32,
    pub version_conflicts: u32,
    pub noops: u32,
    pub throttled_millis: u32,
    pub requests_per_second: f32,
    pub throttled_until_millis: u32,
    pub failures: Vec<serde_json::Value>,
}

impl Default for UpdateByQueryBody {
    fn default() -> Self {
        UpdateByQueryBody {
            took: 0,
            timed_out: false,
            total: 0,
            updated: 0,
            deleted: 0,
            batches: 0,
            version_conflicts: 0,
            noops: 0,
            throttled_millis: 0,
            requests_per_second: -1.0,
            throttled_until_millis: 0,
            failures: Vec::new(),
        }
    }
}

#[derive(Serialize)]
pub struct UpdateByQueryAsync {
    pub task: String,
}

// Kibana runs `_update_by_query` async: we hand back a task id and report it already complete on
// the first `_tasks` poll, since the stub did no work.
pub const STUB_TASK_ID: &str = "topk-es:0";

#[derive(Serialize)]
pub struct TaskStatus {
    pub completed: bool,
    pub task: TaskInfo,
    pub response: UpdateByQueryBody,
}

#[derive(Serialize)]
pub struct TaskInfo {
    pub node: &'static str,
    pub id: u64,
    pub action: &'static str,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus {
            completed: true,
            task: TaskInfo {
                node: "topk-es",
                id: 0,
                action: "indices:data/write/update/byquery",
            },
            response: UpdateByQueryBody::default(),
        }
    }
}


impl Default for Shards {
    fn default() -> Self {
        Self {
            total: 1,
            successful: 1,
            failed: 0,
        }
    }
}
