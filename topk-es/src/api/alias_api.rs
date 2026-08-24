use serde::{Deserialize, Serialize};

// `POST /_aliases`. Aliases are derived from index names (see api::alias), so the happy path —
// Kibana pointing `.kibana` at the sole `.kibana_<v>_001` — is already what resolution returns.
// We accept the actions and acknowledge; `remove_index` is the one that must do real work.
#[derive(Deserialize)]
pub struct AliasActions {
    pub actions: Vec<AliasAction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasAction {
    Add(AliasTarget),
    Remove(AliasTarget),
    RemoveIndex { index: super::IndexName },
}

#[derive(Deserialize)]
pub struct AliasTarget {
    #[allow(dead_code)]
    pub index: String,
    #[allow(dead_code)]
    pub alias: String,
}

#[derive(Serialize)]
pub struct AliasAck {
    pub acknowledged: bool,
}
