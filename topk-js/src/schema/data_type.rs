use napi_derive::napi;

#[napi(string_enum, namespace = "schema")]
#[derive(Clone, Debug)]
pub enum DataType {
    Text,
    Integer,
    Float,
    Boolean,
    F32Vector { dimension: u32 },
    U8Vector { dimension: u32 },
    BinaryVector { dimension: u32 },
    F32SparseVector,
    U8SparseVector,
    Bytes,
}

impl From<topk_rs::proto::v1::control::FieldType> for DataType {
    fn from(field_type: topk_rs::proto::v1::control::FieldType) -> Self {
        match field_type.data_type {
            Some(data_type) => match data_type {
                topk_rs::proto::v1::control::field_type::DataType::Text(_) => DataType::Text,
                topk_rs::proto::v1::control::field_type::DataType::Integer(_) => DataType::Integer,
                topk_rs::proto::v1::control::field_type::DataType::Float(_) => DataType::Float,
                topk_rs::proto::v1::control::field_type::DataType::Boolean(_) => DataType::Boolean,
                topk_rs::proto::v1::control::field_type::DataType::F32Vector(vector) => {
                    DataType::F32Vector {
                        dimension: vector.dimension,
                    }
                }
                topk_rs::proto::v1::control::field_type::DataType::U8Vector(vector) => {
                    DataType::U8Vector {
                        dimension: vector.dimension,
                    }
                }
                topk_rs::proto::v1::control::field_type::DataType::BinaryVector(vector) => {
                    DataType::BinaryVector {
                        dimension: vector.dimension,
                    }
                }
                topk_rs::proto::v1::control::field_type::DataType::F32SparseVector(_) => {
                    DataType::F32SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::U8SparseVector(_) => {
                    DataType::U8SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::Bytes(_) => DataType::Bytes,
            },
            None => unreachable!("Invalid data type proto"),
        }
    }
}
