use napi_derive::napi;

/// @ignore
#[napi(string_enum, namespace = "schema")]
#[derive(Clone, Debug)]
pub enum DataType {
    Text,
    Integer,
    Float,
    Boolean,
    F8Vector {
        dimension: u32,
    },
    F16Vector {
        dimension: u32,
    },
    F32Vector {
        dimension: u32,
    },
    U8Vector {
        dimension: u32,
    },
    I8Vector {
        dimension: u32,
    },
    BinaryVector {
        dimension: u32,
    },
    F32SparseVector,
    U8SparseVector,
    Bytes,
    List {
        value_type: ListValueType,
    },
    Matrix {
        dimension: u32,
        value_type: MatrixValueType,
    },
}

#[napi(string_enum = "lowercase", namespace = "schema")]
#[derive(Clone, Debug)]
pub enum ListValueType {
    Text,
    Integer,
    Float,
}

#[napi(string_enum = "lowercase", namespace = "schema")]
#[derive(Clone, Debug)]
pub enum MatrixValueType {
    F32,
    F16,
    F8,
    U8,
    I8,
}

impl From<MatrixValueType> for topk_rs::proto::v1::control::field_type_matrix::MatrixValueType {
    fn from(value: MatrixValueType) -> Self {
        match value {
            MatrixValueType::F32 => {
                topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F32
            }
            MatrixValueType::F16 => {
                topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F16
            }
            MatrixValueType::F8 => {
                topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F8
            }
            MatrixValueType::U8 => {
                topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::U8
            }
            MatrixValueType::I8 => {
                topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::I8
            }
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::field_type_matrix::MatrixValueType> for MatrixValueType {
    type Error = crate::error::TopkError;

    fn try_from(
        value: topk_rs::proto::v1::control::field_type_matrix::MatrixValueType,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F32 => {
                Ok(MatrixValueType::F32)
            }
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F16 => {
                Ok(MatrixValueType::F16)
            }
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::F8 => {
                Ok(MatrixValueType::F8)
            }
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::U8 => {
                Ok(MatrixValueType::U8)
            }
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::I8 => {
                Ok(MatrixValueType::I8)
            }
            topk_rs::proto::v1::control::field_type_matrix::MatrixValueType::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}

impl From<ListValueType> for topk_rs::proto::v1::control::FieldTypeList {
    fn from(value: ListValueType) -> Self {
        match value {
            ListValueType::Integer => topk_rs::proto::v1::control::FieldTypeList {
                value_type: topk_rs::proto::v1::control::field_type_list::ListValueType::Integer
                    as i32,
            },
            ListValueType::Float => topk_rs::proto::v1::control::FieldTypeList {
                value_type: topk_rs::proto::v1::control::field_type_list::ListValueType::Float
                    as i32,
            },
            ListValueType::Text => topk_rs::proto::v1::control::FieldTypeList {
                value_type: topk_rs::proto::v1::control::field_type_list::ListValueType::String
                    as i32,
            },
        }
    }
}

impl TryFrom<topk_rs::proto::v1::control::field_type_list::ListValueType> for ListValueType {
    type Error = crate::error::TopkError;

    fn try_from(
        value: topk_rs::proto::v1::control::field_type_list::ListValueType,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            topk_rs::proto::v1::control::field_type_list::ListValueType::Integer => {
                Ok(ListValueType::Integer)
            }
            topk_rs::proto::v1::control::field_type_list::ListValueType::Float => {
                Ok(ListValueType::Float)
            }
            topk_rs::proto::v1::control::field_type_list::ListValueType::String => {
                Ok(ListValueType::Text)
            }
            topk_rs::proto::v1::control::field_type_list::ListValueType::Unspecified => {
                Err(topk_rs::Error::InvalidProto.into())
            }
        }
    }
}
impl TryFrom<topk_rs::proto::v1::control::FieldType> for DataType {
    type Error = crate::error::TopkError;

    fn try_from(
        mut field_type: topk_rs::proto::v1::control::FieldType,
    ) -> std::result::Result<Self, Self::Error> {
        let data_type = field_type
            .data_type
            .take()
            .ok_or(topk_rs::Error::InvalidProto)?;
        Ok(match data_type {
            topk_rs::proto::v1::control::field_type::DataType::Text(_) => DataType::Text,
            topk_rs::proto::v1::control::field_type::DataType::Integer(_) => DataType::Integer,
            topk_rs::proto::v1::control::field_type::DataType::Float(_) => DataType::Float,
            topk_rs::proto::v1::control::field_type::DataType::Boolean(_) => DataType::Boolean,
            topk_rs::proto::v1::control::field_type::DataType::F8Vector(vector) => {
                DataType::F8Vector {
                    dimension: vector.dimension,
                }
            }
            topk_rs::proto::v1::control::field_type::DataType::F16Vector(vector) => {
                DataType::F16Vector {
                    dimension: vector.dimension,
                }
            }
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
            topk_rs::proto::v1::control::field_type::DataType::I8Vector(vector) => {
                DataType::I8Vector {
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
            topk_rs::proto::v1::control::field_type::DataType::List(list) => DataType::List {
                value_type: list.value_type().try_into()?,
            },
            topk_rs::proto::v1::control::field_type::DataType::Matrix(matrix) => DataType::Matrix {
                dimension: matrix.dimension,
                value_type: matrix.value_type().try_into()?,
            },
        })
    }
}
