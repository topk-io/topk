use std::path::PathBuf;

use topk_rs::proto::v1::data::Document;

pub mod books;
pub mod multi_vec;
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

#[allow(dead_code)]
pub fn test_pdf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("utils")
        .join("dataset")
        .join("pdfko.pdf")
}
