use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use topk_protos::v1::control::{self, KeywordIndexType};

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

#[napi(string_enum)]
pub enum VectorFieldIndexMetric {
  Cosine,
  Euclidean,
  DotProduct,
  Hamming,
}

impl From<i32> for VectorFieldIndexMetric {
  fn from(metric: i32) -> Self {
    match metric {
      1 => VectorFieldIndexMetric::Cosine,
      2 => VectorFieldIndexMetric::Euclidean,
      3 => VectorFieldIndexMetric::DotProduct,
      4 => VectorFieldIndexMetric::Hamming,
      _ => unreachable!("Unsupported vector field index metric"),
    }
  }
}

impl From<VectorFieldIndexMetric> for i32 {
  fn from(metric: VectorFieldIndexMetric) -> Self {
    match metric {
      VectorFieldIndexMetric::Cosine => 1,
      VectorFieldIndexMetric::Euclidean => 2,
      VectorFieldIndexMetric::DotProduct => 3,
      VectorFieldIndexMetric::Hamming => 4,
    }
  }
}

#[napi(string_enum)]
pub enum EmbeddingDataType {
  F32,
  U8,
  /// Binary quantized uint8
  Binary,
}

impl From<Option<i32>> for EmbeddingDataType {
  fn from(embedding_type: Option<i32>) -> Self {
    match embedding_type {
      Some(0) => EmbeddingDataType::F32,
      Some(1) => EmbeddingDataType::U8,
      Some(2) => EmbeddingDataType::Binary,
      // Default to F32 for unspecified embedding type
      _ => EmbeddingDataType::F32,
    }
  }
}

impl From<EmbeddingDataType> for Option<i32> {
  fn from(embedding_type: EmbeddingDataType) -> Self {
    match embedding_type {
      EmbeddingDataType::F32 => Some(0),
      EmbeddingDataType::U8 => Some(1),
      EmbeddingDataType::Binary => Some(2),
    }
  }
}

#[napi]
pub enum FieldIndex {
  Keyword,
  Vector {
    metric: VectorFieldIndexMetric,
  },
  Semantic {
    model: Option<String>,
    embedding_type: EmbeddingDataType,
  },
}

impl From<control::FieldIndex> for FieldIndex {
  fn from(field_index: control::FieldIndex) -> Self {
    match field_index.index.unwrap_or_else(|| {
      control::field_index::Index::KeywordIndex(control::KeywordIndex {
        index_type: KeywordIndexType::Text.into(),
      })
    }) {
      control::field_index::Index::KeywordIndex(_k) => FieldIndex::Keyword {},
      control::field_index::Index::VectorIndex(v) => FieldIndex::Vector {
        metric: v.metric.into(),
      },
      control::field_index::Index::SemanticIndex(s) => FieldIndex::Semantic {
        model: s.model,
        embedding_type: s.embedding_type.into(),
      },
    }
  }
}

impl From<FieldIndex> for control::FieldIndex {
  fn from(field_index: FieldIndex) -> Self {
    Self {
      index: Some(match field_index {
        FieldIndex::Keyword => control::field_index::Index::KeywordIndex(control::KeywordIndex {
          index_type: KeywordIndexType::Text.into(),
        }),
        FieldIndex::Vector { metric } => {
          control::field_index::Index::VectorIndex(control::VectorIndex {
            metric: metric.into(),
          })
        }
        FieldIndex::Semantic {
          model,
          embedding_type,
        } => control::field_index::Index::SemanticIndex(control::SemanticIndex {
          model,
          embedding_type: embedding_type.into(),
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
      None => unreachable!("Unsupported field type None"),
    }
  }
}

#[napi(object)]
pub struct CreateCollectionOptions {
  pub name: String,
  pub schema: HashMap<String, FieldSpec>,
}
