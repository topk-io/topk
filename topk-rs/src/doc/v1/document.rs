use std::collections::HashMap;

use rkyv::{Archive, Deserialize, Serialize};

use super::Value;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct Document {
    pub fields: HashMap<String, Value>,
}
