use crate::Error;
use super::{CreateDatasetResponse, Dataset, GetDatasetResponse};

impl CreateDatasetResponse {
    pub fn dataset(&self) -> Result<&Dataset, Error> {
        self.dataset.as_ref().ok_or(Error::InvalidProto)
    }
}

impl GetDatasetResponse {
    pub fn dataset(&self) -> Result<&Dataset, Error> {
        self.dataset.as_ref().ok_or(Error::InvalidProto)
    }
}
