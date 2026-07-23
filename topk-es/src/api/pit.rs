use serde::{Deserialize, Serialize};

use super::IndexName;
use crate::Error;

// Not a point in time: the id names an index, it does not freeze one. See ELASTIC.md — paging is
// ordered but not isolated from concurrent writes.
#[derive(Deserialize)]
pub struct PitRef {
    pub id: String,

    #[serde(default)]
    #[allow(dead_code)]
    pub keep_alive: Option<String>,
}

#[derive(Serialize)]
pub struct PitOpened {
    pub id: String,
}

#[derive(Serialize)]
pub struct PitClosed {
    pub succeeded: bool,
    pub num_freed: u32,
}

#[derive(Deserialize)]
pub struct PitClose {
    pub id: String,
}

impl PitRef {
    pub fn index(&self) -> Result<IndexName, Error> {
        IndexName::try_from(self.id.clone())
            .map_err(|_| Error::BadRequest(format!("malformed point-in-time id [{}]", self.id)))
    }
}
