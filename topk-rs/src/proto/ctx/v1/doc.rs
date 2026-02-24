#[derive(Clone)]
pub struct DocId(String);

impl From<DocId> for String {
    fn from(doc_id: DocId) -> Self {
        doc_id.0
    }
}

impl From<String> for DocId {
    fn from(s: String) -> Self {
        DocId(s)
    }
}

impl From<&str> for DocId {
    fn from(s: &str) -> Self {
        DocId(s.to_string())
    }
}
