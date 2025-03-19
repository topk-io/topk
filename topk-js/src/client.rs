use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use topk_protos::v1::control;

#[napi(string_enum)]
pub enum DataType {
  Text,
  Integer,
  Float,
  Boolean,
  F32Vector,
  U8Vector,
  BinaryVector,
  Bytes,
}

#[napi(object)]
pub struct KeywordIndex {
  pub index_type: i32,
}

#[napi(object)]
pub struct VectorIndex {
  /// Distance metric
  pub metric: i32,
}

#[napi(object)]
pub struct SemanticIndex {
  /// Model to be used for embedding text to vectors.
  pub model: ::core::option::Option<String>,
  /// Data type of the embedding vectors.
  pub embedding_type: ::core::option::Option<i32>,
}

#[napi]
pub enum FieldIndex {
  Keyword {
    index_type: i32,
  },
  Vector {
    metric: i32,
  },
  Semantic {
    model: Option<String>,
    embedding_type: Option<i32>,
  },
}

impl From<control::FieldIndex> for FieldIndex {
  fn from(field_index: control::FieldIndex) -> Self {
    match field_index.index.unwrap_or_else(|| {
      control::field_index::Index::KeywordIndex(control::KeywordIndex { index_type: 0 })
    }) {
      control::field_index::Index::KeywordIndex(k) => FieldIndex::Keyword {
        index_type: k.index_type,
      },
      control::field_index::Index::VectorIndex(v) => FieldIndex::Vector { metric: v.metric },
      control::field_index::Index::SemanticIndex(s) => FieldIndex::Semantic {
        model: s.model,
        embedding_type: s.embedding_type,
      },
    }
  }
}

impl From<FieldIndex> for control::FieldIndex {
  fn from(field_index: FieldIndex) -> Self {
    Self {
      index: Some(match field_index {
        FieldIndex::Keyword { index_type } => {
          control::field_index::Index::KeywordIndex(control::KeywordIndex { index_type })
        }
        FieldIndex::Vector { metric } => {
          control::field_index::Index::VectorIndex(control::VectorIndex { metric })
        }
        FieldIndex::Semantic {
          model,
          embedding_type,
        } => control::field_index::Index::SemanticIndex(control::SemanticIndex {
          model,
          embedding_type,
        }),
      }),
    }
  }
}

#[napi(object)]
pub struct FieldSpec {
  pub data_type: DataType,
  pub required: bool,
  pub index: Option<FieldIndex>,
}

#[napi(object)]
pub struct Schema {
  pub schema: HashMap<String, String>,
}

#[napi(object)]
pub struct ClientConfig {
  pub api_key: String,
  pub region: String,
  pub host: Option<String>,
  pub https: Option<bool>,
}

#[napi(object)]
pub struct Collection {
  pub name: String,
  pub org_id: String,
  pub project_id: String,
  pub schema: HashMap<String, FieldSpec>,
  pub region: String,
}

impl From<control::Collection> for Collection {
  fn from(collection: control::Collection) -> Self {
    Self {
      name: collection.name,
      org_id: collection.org_id,
      project_id: collection.project_id,
      schema: collection
        .schema
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect(),
      region: collection.region,
    }
  }
}

impl From<control::FieldSpec> for FieldSpec {
  fn from(field_spec: control::FieldSpec) -> Self {
    Self {
      data_type: DataType::from(field_spec.data_type.unwrap_or_default()),
      required: field_spec.required,
      index: field_spec.index.map(|idx| idx.into()),
    }
  }
}

impl From<FieldSpec> for control::FieldSpec {
  fn from(field_spec: FieldSpec) -> Self {
    Self {
      data_type: Some(control::FieldType {
        data_type: Some(match field_spec.data_type {
          DataType::Text => control::field_type::DataType::Text(Default::default()),
          DataType::Integer => control::field_type::DataType::Integer(Default::default()),
          DataType::Float => control::field_type::DataType::Float(Default::default()),
          DataType::Boolean => control::field_type::DataType::Boolean(Default::default()),
          DataType::F32Vector => control::field_type::DataType::F32Vector(Default::default()),
          DataType::U8Vector => control::field_type::DataType::U8Vector(Default::default()),
          DataType::BinaryVector => control::field_type::DataType::BinaryVector(Default::default()),
          DataType::Bytes => control::field_type::DataType::Bytes(Default::default()),
        }),
      }),
      required: field_spec.required,
      index: field_spec.index.map(|idx| idx.into()),
    }
  }
}

impl From<control::FieldType> for DataType {
  fn from(field_type: control::FieldType) -> Self {
    match field_type.data_type {
      Some(data_type) => match data_type {
        control::field_type::DataType::Text(_) => DataType::Text,
        control::field_type::DataType::Integer(_) => DataType::Integer,
        control::field_type::DataType::Float(_) => DataType::Float,
        control::field_type::DataType::Boolean(_) => DataType::Boolean,
        control::field_type::DataType::F32Vector(_) => DataType::F32Vector,
        control::field_type::DataType::U8Vector(_) => DataType::U8Vector,
        control::field_type::DataType::BinaryVector(_) => DataType::BinaryVector,
        control::field_type::DataType::Bytes(_) => DataType::Bytes,
      },
      None => DataType::Text, // Default to Text type if none specified
    }
  }
}

#[napi(object)]
pub struct CreateCollectionOptions {
  pub name: String,
  pub schema: HashMap<String, FieldSpec>,
}

pub struct TopkError(topk_rs::Error);

impl From<topk_rs::Error> for TopkError {
  fn from(error: topk_rs::Error) -> Self {
    TopkError(error)
  }
}

impl From<TopkError> for napi::Error {
  fn from(error: TopkError) -> Self {
    napi::Error::new(
      napi::Status::GenericFailure,
      format!("failed to create collection: {:?}", error.0),
    )
  }
}

#[napi]
pub struct CollectionsClient {
  client: Arc<topk_rs::Client>,
}

#[napi]
impl CollectionsClient {
  pub fn new(client: Arc<topk_rs::Client>) -> Self {
    Self { client }
  }

  #[napi]
  pub async fn list(&self) -> Result<Vec<Collection>> {
    let collections = self
      .client
      .collections()
      .list()
      .await
      .map_err(TopkError::from)?;
    let collections_napi = collections.into_iter().map(|c| c.into()).collect();
    Ok(collections_napi)
  }

  #[napi]
  pub async fn create(&self, options: CreateCollectionOptions) -> Result<Collection> {
    let proto_schema: HashMap<String, control::FieldSpec> = options
      .schema
      .into_iter()
      .map(|(k, v)| (k, v.into()))
      .collect();

    let collection = self
      .client
      .collections()
      .create(options.name, proto_schema)
      .await
      .map_err(TopkError::from)?;

    Ok(collection.into())
  }
}
