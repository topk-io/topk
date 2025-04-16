#[derive(Debug, Clone)]
pub enum Vector {
    F32(Vec<f32>),
    U8(Vec<u8>),
}

impl Vector {
    pub fn float(values: Vec<f32>) -> Self {
        Self::F32(values)
    }

    pub fn byte(values: Vec<u8>) -> Self {
        Self::U8(values)
    }
}

impl Into<topk_protos::v1::data::Vector> for Vector {
    fn into(self) -> topk_protos::v1::data::Vector {
        match self {
            Vector::F32(values) => topk_protos::v1::data::Vector::float(values),
            Vector::U8(values) => topk_protos::v1::data::Vector::byte(values),
        }
    }
}

impl From<topk_protos::v1::data::Vector> for Vector {
    fn from(vector: topk_protos::v1::data::Vector) -> Self {
        match vector.vector {
            Some(topk_protos::v1::data::vector::Vector::Float(values)) => {
                Vector::F32(values.values)
            }
            Some(topk_protos::v1::data::vector::Vector::Byte(values)) => Vector::U8(values.values),
            t => panic!("Invalid vector type: {:?}", t),
        }
    }
}

impl From<Vec<f32>> for Vector {
    fn from(values: Vec<f32>) -> Self {
        Vector::F32(values)
    }
}

#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: Vector },
    SemanticSimilarity { field: String, query: String },
}

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_protos::v1::data::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => topk_protos::v1::data::FunctionExpr::bm25_score(),
            FunctionExpr::VectorScore { field, query } => {
                topk_protos::v1::data::FunctionExpr::vector_distance(field, query.into())
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_protos::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
