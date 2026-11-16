use anyhow::Result;

use crate::management::proto::Project;
use crate::ProjectId;

pub mod proto {
    tonic::include_proto!("topk.management.v1");
}

impl Project {
    pub fn project_id(&self) -> Result<ProjectId> {
        self.project_id.parse()
    }
}
