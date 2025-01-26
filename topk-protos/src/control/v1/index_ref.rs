use crate::{OrgId, ProjectId};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct IndexRef {
    pub org_id: OrgId,
    pub project_id: ProjectId,
    pub internal_id: uuid::Uuid,
}

impl IndexRef {
    pub fn new(org_id: OrgId, project_id: ProjectId, internal_id: uuid::Uuid) -> Self {
        IndexRef {
            org_id,
            project_id,
            internal_id,
        }
    }

    pub fn org_id(&self) -> OrgId {
        self.org_id
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn internal_id(&self) -> Uuid {
        self.internal_id
    }

    /// Returns the data path form this index in the following form:
    /// `/org/{org_id}/proj/{project_id}/col/{internal_id}`
    pub fn data_path(&self) -> String {
        format!(
            "/org/{:x}/proj/{:x}/col/{:x}",
            self.org_id, self.project_id, self.internal_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_path() {
        // IMPORTANT
        // !!! DO NOT CHANGE THIS TEST !!!
        // This test checks that index data path formatting stays consistent. If this test fails,
        // then path formatting has changed and bad stuff will happen.
        let internal_id = Uuid::from_bytes([
            20, 255, 54, 90, 79, 15, 77, 115, 140, 74, 204, 117, 40, 34, 91, 173,
        ]);
        let index = IndexRef::new(123.into(), 456.into(), internal_id);
        assert_eq!(
            index.data_path(),
            format!("/org/7b/proj/1c8/col/14ff365a-4f0f-4d73-8c4a-cc7528225bad")
        );
    }
}
