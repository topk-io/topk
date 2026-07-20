use std::collections::HashMap;

use serde::Serialize;

use super::mapping::MappingProperties;

#[derive(Serialize)]
pub struct IndexCreatedBody {
    pub acknowledged: bool,
    pub shards_acknowledged: bool,
    pub index: String,
}

impl IndexCreatedBody {
    pub fn new(index: String) -> Self {
        Self {
            acknowledged: true,
            shards_acknowledged: true,
            index,
        }
    }
}

#[derive(Serialize)]
pub struct AcknowledgedBody {
    pub acknowledged: bool,
}

#[derive(Serialize)]
pub struct MappingBody {
    pub properties: MappingProperties,
}

#[derive(Serialize)]
pub struct MappingIndexBody {
    pub mappings: MappingBody,
}

impl MappingIndexBody {
    pub fn new(properties: MappingProperties) -> Self {
        Self {
            mappings: MappingBody { properties },
        }
    }
}

#[derive(Serialize)]
pub struct IndexSettingsInnerBody {
    pub provided_name: String,
    pub number_of_shards: &'static str,
    pub number_of_replicas: &'static str,
}

#[derive(Serialize)]
pub struct IndexSettingsBody {
    pub index: IndexSettingsInnerBody,
}

#[derive(Serialize)]
pub struct GetIndexBody {
    pub aliases: HashMap<String, serde_json::Value>,
    pub mappings: MappingBody,
    pub settings: IndexSettingsBody,
}

impl GetIndexBody {
    pub fn new(index: String, properties: MappingProperties) -> Self {
        Self {
            aliases: HashMap::new(),
            mappings: MappingBody { properties },
            settings: IndexSettingsBody {
                index: IndexSettingsInnerBody {
                    provided_name: index,
                    number_of_shards: "1",
                    number_of_replicas: "0",
                },
            },
        }
    }
}
