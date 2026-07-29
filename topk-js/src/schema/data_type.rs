use std::collections::HashMap;

use napi_derive::napi;

use topk_rs::proto::v1::control::field_type_list::ListValueType as ListValueTypePb;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType as MatrixValueTypePb;

use crate::schema::field_spec::FieldSpec;

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
    F16SparseVector,
    F8SparseVector,
    I8SparseVector,
    U8SparseVector,
    Bytes,
    List {
        value_type: ListValueType,
    },
    Struct {
        fields: HashMap<String, FieldSpec>,
    },
    Matrix {
        dimension: u32,
        value_type: MatrixValueType,
    },
    Unknown,
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

impl From<topk_rs::proto::v1::control::FieldType> for DataType {
    fn from(field_type: topk_rs::proto::v1::control::FieldType) -> Self {
        match field_type.data_type {
            Some(data_type) => match data_type {
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
                topk_rs::proto::v1::control::field_type::DataType::F16SparseVector(_) => {
                    DataType::F16SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::F8SparseVector(_) => {
                    DataType::F8SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::I8SparseVector(_) => {
                    DataType::I8SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::U8SparseVector(_) => {
                    DataType::U8SparseVector
                }
                topk_rs::proto::v1::control::field_type::DataType::Bytes(_) => DataType::Bytes,
                topk_rs::proto::v1::control::field_type::DataType::List(list) => DataType::List {
                    value_type: match list.value_type() {
                        ListValueTypePb::Integer => ListValueType::Integer,
                        ListValueTypePb::Float => ListValueType::Float,
                        ListValueTypePb::String => ListValueType::Text,
                        ListValueTypePb::Unspecified => return DataType::Unknown,
                    },
                },
                topk_rs::proto::v1::control::field_type::DataType::Struct(s) => DataType::Struct {
                    fields: s.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
                },
                topk_rs::proto::v1::control::field_type::DataType::Matrix(matrix) => {
                    DataType::Matrix {
                        dimension: matrix.dimension,
                        value_type: match matrix.value_type() {
                            MatrixValueTypePb::F32 => MatrixValueType::F32,
                            MatrixValueTypePb::F16 => MatrixValueType::F16,
                            MatrixValueTypePb::F8 => MatrixValueType::F8,
                            MatrixValueTypePb::U8 => MatrixValueType::U8,
                            MatrixValueTypePb::I8 => MatrixValueType::I8,
                            MatrixValueTypePb::Unspecified => return DataType::Unknown,
                        },
                    }
                }
                topk_rs::proto::v1::control::field_type::DataType::Timestamp(_) => {
                    unimplemented!("timestamp: see #530")
                }
            },
            None => DataType::Unknown,
        }
    }
}
