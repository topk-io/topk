use std::str::FromStr;

use crate::Error;

use super::*;

impl FieldIndex {
    pub fn keyword(index_type: KeywordIndexType) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::KeywordIndex(KeywordIndex {
                index_type: index_type.into(),
            })),
        }
    }

    pub fn vector(metric: VectorDistanceMetric) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::VectorIndex(VectorIndex {
                metric: metric.into(),
                exact: None,
            })),
        }
    }

    pub fn multi_vector(
        metric: MultiVectorDistanceMetric,
        quantization: Option<MultiVectorQuantization>,
        width: Option<u32>,
        top_k: Option<u32>,
    ) -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex {
                metric: metric.into(),
                #[allow(deprecated)]
                sketch_bits: None,
                quantization: quantization.map(|q| q.into()),
                width,
                top_k,
                skip_smve: false,
                encoding_version: 0,
            })),
        }
    }

    pub fn semantic() -> FieldIndex {
        FieldIndex {
            index: Some(field_index::Index::SemanticIndex(SemanticIndex {
                #[allow(deprecated)]
                model: None,
                #[allow(deprecated)]
                embedding_type: None,
            })),
        }
    }

    /// Skip sparse multi-vector encoding (SMVE) for multi-vector index.
    pub fn skip_smve(mut self) -> Self {
        if let Some(field_index::Index::MultiVectorIndex(mvi)) = self.index.as_mut() {
            mvi.skip_smve = true;
        }
        self
    }
}

impl FromStr for VectorDistanceMetric {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cosine" => Ok(Self::Cosine),
            "euclidean" => Ok(Self::Euclidean),
            "dot_product" => Ok(Self::DotProduct),
            "hamming" => Ok(Self::Hamming),
            other => Err(Error::InvalidArgument(format!(
                "invalid vector distance metric `{other}`, expected: cosine | euclidean | dot_product | hamming"
            ))),
        }
    }
}

impl FromStr for MultiVectorDistanceMetric {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "maxsim" => Ok(Self::Maxsim),
            other => Err(Error::InvalidArgument(format!(
                "invalid multi-vector distance metric `{other}`, expected: maxsim"
            ))),
        }
    }
}

impl FromStr for MultiVectorQuantization {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "1bit" => Ok(Self::Binary1bit),
            "2bit" => Ok(Self::Binary2bit),
            "scalar" => Ok(Self::Scalar),
            other => Err(Error::InvalidArgument(format!(
                "invalid multi-vector quantization `{other}`, expected: 1bit | 2bit | scalar"
            ))),
        }
    }
}
