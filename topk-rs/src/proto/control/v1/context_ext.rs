use crate::proto::v1::ctx::{Content, Image, Page, SearchResult};
use crate::Error;

impl SearchResult {
    pub fn content(&self) -> Result<&Content, Error> {
        self.content.as_ref().ok_or(Error::InvalidProto)
    }
}

impl Page {
    pub fn image(&self) -> Result<&Image, Error> {
        self.image.as_ref().ok_or(Error::InvalidProto)
    }
}
