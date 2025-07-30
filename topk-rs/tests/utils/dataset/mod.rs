use topk_rs::proto::v1::data::Document;

pub mod books;
pub mod semantic;

#[allow(dead_code)]
pub trait Dataset {
    /// Returns a reference to the document with the given id.
    fn find_by_id(&self, id: &str) -> Option<&Document>;
}

impl Dataset for Vec<Document> {
    fn find_by_id(&self, id: &str) -> Option<&Document> {
        self.iter().find(|doc| doc.id().unwrap() == id).clone()
    }
}
