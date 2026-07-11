use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use topk_rs::proto::v1::control::{
    field_index, field_type, field_type_matrix::MatrixValueType, FieldIndex, FieldSpec,
    KeywordIndexType, MultiVectorDistanceMetric, MultiVectorQuantization, VectorDistanceMetric,
};

use crate::Error;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexMapping {
    #[serde(default)]
    #[allow(dead_code)]
    settings: Option<serde_json::Value>,

    #[serde(default)]
    mappings: Option<Mappings>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mappings {
    #[serde(default)]
    properties: Option<MappingProperties>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
pub struct MappingProperties(pub HashMap<String, FieldMapping>);

impl MappingProperties {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<HashMap<String, FieldSpec>> for MappingProperties {
    type Error = Error;

    fn try_from(fields: HashMap<String, FieldSpec>) -> Result<Self, Self::Error> {
        fields
            .into_iter()
            .filter(|(name, _)| !name.starts_with('_'))
            .map(|(name, spec)| FieldMapping::try_from(&spec).map(|mapping| (name, mapping)))
            .collect::<Result<HashMap<_, _>, Error>>()
            .map(Self)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum FieldMapping {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        index: Option<bool>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[allow(dead_code)]
        fields: Option<MappingProperties>,
    },

    #[serde(rename = "keyword")]
    Keyword {
        #[serde(default)]
        index: Option<bool>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[allow(dead_code)]
        fields: Option<MappingProperties>,
    },

    #[serde(rename = "integer", alias = "long", alias = "short", alias = "byte")]
    Integer {
        #[serde(default)]
        #[allow(dead_code)]
        index: Option<bool>,
    },

    #[serde(rename = "float", alias = "double", alias = "half_float")]
    Float {
        #[serde(default)]
        #[allow(dead_code)]
        index: Option<bool>,
    },

    #[serde(rename = "boolean")]
    Boolean {
        #[serde(default)]
        #[allow(dead_code)]
        index: Option<bool>,
    },

    #[serde(rename = "object", alias = "nested")]
    Object {
        #[serde(default, skip_serializing_if = "MappingProperties::is_empty")]
        properties: MappingProperties,
    },

    #[serde(rename = "dense_vector")]
    DenseVector {
        dims: u32,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        similarity: Option<Similarity>,

        #[serde(default)]
        index: Option<bool>,

        #[serde(default)]
        element_type: ElementType,
    },

    #[serde(rename = "rank_vectors", alias = "matrix")]
    RankVectors {
        dims: u32,

        #[serde(default)]
        element_type: MatrixElementType,

        #[serde(default)]
        index: Option<bool>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        quantization: Option<Quantization>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_k: Option<u32>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
    },

    #[serde(rename = "semantic_text")]
    SemanticText {
        #[allow(dead_code)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inference_id: Option<String>,

        #[allow(dead_code)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_inference_id: Option<String>,

        #[allow(dead_code)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chunking_settings: Option<serde_json::Value>,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Similarity {
    Cosine,
    DotProduct,
    L2Norm,
}

impl From<Similarity> for VectorDistanceMetric {
    fn from(similarity: Similarity) -> Self {
        match similarity {
            Similarity::Cosine => VectorDistanceMetric::Cosine,
            Similarity::DotProduct => VectorDistanceMetric::DotProduct,
            Similarity::L2Norm => VectorDistanceMetric::Euclidean,
        }
    }
}

impl From<VectorDistanceMetric> for Similarity {
    fn from(metric: VectorDistanceMetric) -> Self {
        match metric {
            VectorDistanceMetric::Cosine | VectorDistanceMetric::Unspecified => Similarity::Cosine,
            VectorDistanceMetric::DotProduct => Similarity::DotProduct,
            VectorDistanceMetric::Euclidean | VectorDistanceMetric::Hamming => Similarity::L2Norm,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    #[default]
    Float,
    Byte,
    Bit,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatrixElementType {
    #[default]
    Float,
    Byte,
}

impl TryFrom<MatrixValueType> for MatrixElementType {
    type Error = Error;

    fn try_from(value: MatrixValueType) -> Result<Self, Self::Error> {
        match value {
            MatrixValueType::F32 => Ok(MatrixElementType::Float),
            MatrixValueType::U8 => Ok(MatrixElementType::Byte),
            _ => Err(Error::Unsupported("Invalid matrix element type".into())),
        }
    }
}

impl From<MatrixElementType> for MatrixValueType {
    fn from(element_type: MatrixElementType) -> Self {
        match element_type {
            MatrixElementType::Float => MatrixValueType::F32,
            MatrixElementType::Byte => MatrixValueType::U8,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Quantization {
    Scalar,
    Binary1Bit,
    Binary2Bit,
}

impl From<Quantization> for MultiVectorQuantization {
    fn from(q: Quantization) -> Self {
        match q {
            Quantization::Scalar => MultiVectorQuantization::Scalar,
            Quantization::Binary1Bit => MultiVectorQuantization::Binary1bit,
            Quantization::Binary2Bit => MultiVectorQuantization::Binary2bit,
        }
    }
}

impl TryFrom<MultiVectorQuantization> for Quantization {
    type Error = Error;

    fn try_from(q: MultiVectorQuantization) -> Result<Self, Self::Error> {
        match q {
            MultiVectorQuantization::Scalar => Ok(Quantization::Scalar),
            MultiVectorQuantization::Binary1bit => Ok(Quantization::Binary1Bit),
            MultiVectorQuantization::Binary2bit => Ok(Quantization::Binary2Bit),
            MultiVectorQuantization::Unspecified => Err(Error::Unsupported(
                "Invalid multi-vector quantization".into(),
            )),
        }
    }
}

impl TryFrom<IndexMapping> for HashMap<String, FieldSpec> {
    type Error = Error;

    fn try_from(mapping: IndexMapping) -> Result<Self, Self::Error> {
        mapping
            .mappings
            .and_then(|m| m.properties)
            .unwrap_or_default()
            .0
            .into_iter()
            .map(|(name, spec)| spec.try_into().map(|field| (name, field)))
            .collect()
    }
}

impl TryFrom<FieldMapping> for FieldSpec {
    type Error = Error;

    fn try_from(mapping: FieldMapping) -> Result<Self, Self::Error> {
        match mapping {
            FieldMapping::Text { index, fields: _ } => {
                let mut field = FieldSpec::text(false);
                if index.unwrap_or(true) {
                    field = field.with_index(FieldIndex::keyword(KeywordIndexType::Text));
                }
                Ok(field)
            }
            FieldMapping::Keyword { index, fields: _ } => {
                let mut field = FieldSpec::text(false);
                if index.unwrap_or(true) {
                    field = field.with_index(FieldIndex::keyword(KeywordIndexType::Exact));
                }
                Ok(field)
            }
            FieldMapping::Integer { index: _ } => Ok(FieldSpec::integer(false)),
            FieldMapping::Float { index: _ } => Ok(FieldSpec::float(false)),
            FieldMapping::Boolean { index: _ } => Ok(FieldSpec::boolean(false)),
            FieldMapping::Object { properties } => Ok(FieldSpec::r#struct(
                false,
                properties
                    .0
                    .into_iter()
                    .map(|(name, spec)| spec.try_into().map(|field| (name, field)))
                    .collect::<Result<HashMap<String, FieldSpec>, Error>>()?,
            )),
            FieldMapping::DenseVector {
                dims,
                similarity,
                index,
                element_type,
            } => {
                let metric = similarity
                    .map(VectorDistanceMetric::from)
                    .unwrap_or(VectorDistanceMetric::Cosine);

                let (mut field, metric) = match element_type {
                    ElementType::Float => (FieldSpec::f32_vector(dims, false), metric),
                    ElementType::Byte => (FieldSpec::i8_vector(dims, false), metric),
                    ElementType::Bit => {
                        if dims % 8 != 0 {
                            return Err(Error::Unsupported(
                                "Dense vector \"dims\" must be a multiple of 8 for element_type \"bit\""
                                    .into(),
                            ));
                        }

                        if !matches!(similarity, None | Some(Similarity::L2Norm)) {
                            return Err(Error::Unsupported(
                                "Dense vector element_type \"bit\" requires similarity \"l2_norm\" (or omitted)"
                                    .into(),
                            ));
                        }

                        (
                            FieldSpec::binary_vector(dims / 8, false),
                            VectorDistanceMetric::Hamming,
                        )
                    }
                };

                if index.unwrap_or(true) {
                    field = field.with_index(FieldIndex::vector(metric));
                }

                Ok(field)
            }
            FieldMapping::RankVectors {
                dims,
                element_type,
                index,
                quantization,
                top_k,
                width,
            } => {
                let mut field = FieldSpec::matrix(false, dims, element_type.into());
                if index.unwrap_or(true) {
                    field = field.with_index(FieldIndex::multi_vector(
                        MultiVectorDistanceMetric::Maxsim,
                        quantization.map(Into::into),
                        width,
                        top_k,
                    ));
                }

                Ok(field)
            }
            FieldMapping::SemanticText { .. } => {
                Ok(FieldSpec::text(false).with_index(FieldIndex::semantic()))
            }
        }
    }
}

impl TryFrom<&FieldSpec> for FieldMapping {
    type Error = Error;

    fn try_from(spec: &FieldSpec) -> Result<Self, Self::Error> {
        let data_type = spec.data_type.as_ref().and_then(|t| t.data_type.as_ref());
        let index = spec.index.as_ref().and_then(|i| i.index.as_ref());

        Ok(match data_type {
            Some(field_type::DataType::Text(_)) => match index {
                Some(field_index::Index::SemanticIndex(_)) => FieldMapping::SemanticText {
                    inference_id: None,
                    search_inference_id: None,
                    chunking_settings: None,
                },
                Some(field_index::Index::KeywordIndex(kw)) => match kw.index_type() {
                    KeywordIndexType::Exact => FieldMapping::Keyword {
                        index: Some(true),
                        fields: None,
                    },
                    _ => FieldMapping::Text {
                        index: Some(true),
                        fields: None,
                    },
                },
                None => FieldMapping::Text {
                    index: Some(false),
                    fields: None,
                },
                _ => return Err(Error::Unsupported("Invalid text index".into())),
            },
            Some(field_type::DataType::Integer(_)) => FieldMapping::Integer { index: Some(false) },
            Some(field_type::DataType::Float(_)) => FieldMapping::Float { index: Some(false) },
            Some(field_type::DataType::Boolean(_)) => FieldMapping::Boolean { index: Some(false) },
            Some(field_type::DataType::Struct(s)) => FieldMapping::Object {
                properties: MappingProperties::try_from(s.fields.clone())?,
            },
            Some(
                dt @ (field_type::DataType::F32Vector(_)
                | field_type::DataType::I8Vector(_)
                | field_type::DataType::U8Vector(_)
                | field_type::DataType::BinaryVector(_)),
            ) => {
                let (dims, element_type) = match dt {
                    field_type::DataType::F32Vector(v) => (v.dimension, ElementType::Float),
                    field_type::DataType::I8Vector(v) => (v.dimension, ElementType::Byte),
                    field_type::DataType::U8Vector(v) => (v.dimension, ElementType::Byte),
                    field_type::DataType::BinaryVector(v) => {
                        (v.dimension.saturating_mul(8), ElementType::Bit)
                    }
                    _ => unreachable!(),
                };

                match index {
                    Some(field_index::Index::VectorIndex(vector)) => FieldMapping::DenseVector {
                        dims,
                        similarity: Some(vector.metric().into()),
                        index: Some(true),
                        element_type,
                    },
                    None => FieldMapping::DenseVector {
                        dims,
                        similarity: None,
                        index: Some(false),
                        element_type,
                    },
                    _ => FieldMapping::Object {
                        properties: MappingProperties::default(),
                    },
                }
            }
            Some(field_type::DataType::Matrix(m)) => {
                let Ok(element_type) = m.value_type().try_into() else {
                    return Ok(FieldMapping::Object {
                        properties: MappingProperties::default(),
                    });
                };

                match index {
                    Some(field_index::Index::MultiVectorIndex(multi_vector)) => {
                        FieldMapping::RankVectors {
                            dims: m.dimension,
                            element_type,
                            index: Some(true),
                            quantization: Some(multi_vector.quantization().try_into()?),
                            top_k: multi_vector.top_k,
                            width: multi_vector.width,
                        }
                    }
                    None => FieldMapping::RankVectors {
                        dims: m.dimension,
                        element_type,
                        index: Some(false),
                        quantization: None,
                        top_k: None,
                        width: None,
                    },
                    _ => FieldMapping::Object {
                        properties: MappingProperties::default(),
                    },
                }
            }
            _ => FieldMapping::Object {
                properties: MappingProperties::default(),
            },
        })
    }
}
