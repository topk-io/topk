use topk_es::api::{ElementType, FieldMapping, MatrixElementType};

use crate::import::spec::{Field, Type};

pub fn field(input: &FieldMapping) -> Field {
    let ty = |ty| Field {
        ty,
        ..Default::default()
    };
    match input {
        FieldMapping::Text { .. }
        | FieldMapping::Keyword { .. }
        | FieldMapping::SemanticText { .. } => ty(Type::Text),
        FieldMapping::Integer { .. } => ty(Type::Int),
        FieldMapping::Float { .. } => ty(Type::Float),
        FieldMapping::Boolean { .. } => ty(Type::Bool),
        FieldMapping::Object { .. } => ty(Type::Struct),
        FieldMapping::DenseVector {
            element_type, dims, ..
        } => {
            let (ty, dim) = match element_type {
                ElementType::Float => (Type::F32Vector, *dims),
                ElementType::Byte => (Type::I8Vector, *dims),
                ElementType::Bit => (Type::BinaryVector, dims / 8),
            };
            Field {
                ty,
                dim: Some(dim),
                ..Default::default()
            }
        }
        FieldMapping::RankVectors {
            element_type, dims, ..
        } => {
            let ty = match element_type {
                MatrixElementType::Float => Type::F32Matrix,
                MatrixElementType::Byte => Type::U8Matrix,
            };
            Field {
                ty,
                cols: Some(*dims),
                ..Default::default()
            }
        }
    }
}
