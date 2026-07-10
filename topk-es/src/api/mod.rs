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
mod path;
mod query;
mod refresh;
mod search;
mod source;
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
pub use write::*;

#[derive(Clone, Serialize)]
pub struct Shards {
    pub total: u32,
    pub successful: u32,
    pub failed: u32,
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
