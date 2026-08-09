use serde::Serialize;

use super::IndexName;

#[derive(Serialize)]
pub struct ResolveIndexBody {
    pub indices: Vec<ResolvedIndex>,
    pub aliases: Vec<ResolvedAlias>,
    pub data_streams: Vec<ResolvedDataStream>,
}

#[derive(Serialize)]
pub struct ResolvedIndex {
    pub name: IndexName,
    pub attributes: Vec<&'static str>,
    pub mode: &'static str,
}

#[derive(Serialize)]
pub struct ResolvedAlias {
    pub name: String,
    pub indices: Vec<IndexName>,
}

#[derive(Serialize)]
pub struct ResolvedDataStream {
    pub name: String,
    pub backing_indices: Vec<IndexName>,
    pub timestamp_field: String,
}

impl From<IndexName> for ResolvedIndex {
    fn from(name: IndexName) -> Self {
        Self {
            name,
            attributes: vec!["open"],
            mode: "standard",
        }
    }
}

impl FromIterator<IndexName> for ResolveIndexBody {
    fn from_iter<I: IntoIterator<Item = IndexName>>(names: I) -> Self {
        Self {
            indices: names.into_iter().map(ResolvedIndex::from).collect(),
            aliases: vec![],
            data_streams: vec![],
        }
    }
}
