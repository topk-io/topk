include!(concat!(env!("OUT_DIR"), "/topk.control.v1.rs"));

pub mod data_type_ext;
pub mod datasets_ext;
pub mod field_index_ext;
pub mod field_spec_ext;

impl field_type::DataType {
    pub fn to_user_friendly_type_name(&self) -> String {
        match self {
            field_type::DataType::Text(..) => "text".to_string(),
            field_type::DataType::Integer(..) => "integer".to_string(),
            field_type::DataType::Float(..) => "float".to_string(),
            field_type::DataType::Boolean(..) => "boolean".to_string(),
            // Dense vector
            field_type::DataType::F32Vector(..) => "vector<f32>".to_string(),
            field_type::DataType::F16Vector(..) => "vector<f16>".to_string(),
            field_type::DataType::F8Vector(..) => "vector<f8>".to_string(),
            field_type::DataType::U8Vector(..) => "vector<u8>".to_string(),
            field_type::DataType::I8Vector(..) => "vector<i8>".to_string(),
            field_type::DataType::BinaryVector(..) => "vector<binary>".to_string(),
            // Sparse vector
            field_type::DataType::F32SparseVector(..) => "sparse_vector<f32>".to_string(),
            field_type::DataType::F16SparseVector(..) => "sparse_vector<f16>".to_string(),
            field_type::DataType::F8SparseVector(..) => "sparse_vector<f8>".to_string(),
            field_type::DataType::I8SparseVector(..) => "sparse_vector<i8>".to_string(),
            field_type::DataType::U8SparseVector(..) => "sparse_vector<u8>".to_string(),
            // Bytes
            field_type::DataType::Bytes(..) => "bytes".to_string(),
            // List
            field_type::DataType::List(list) => match list.value_type() {
                field_type_list::ListValueType::Integer => "list<integer>".to_string(),
                field_type_list::ListValueType::Float => "list<float>".to_string(),
                field_type_list::ListValueType::String => "list<string>".to_string(),
                field_type_list::ListValueType::Unspecified => "list<_>".to_string(),
            },
            // Matrix
            field_type::DataType::Matrix(matrix) => match matrix.value_type() {
                field_type_matrix::MatrixValueType::F32 => "matrix<f32>".to_string(),
                field_type_matrix::MatrixValueType::F16 => "matrix<f16>".to_string(),
                field_type_matrix::MatrixValueType::F8 => "matrix<f8>".to_string(),
                field_type_matrix::MatrixValueType::U8 => "matrix<u8>".to_string(),
                field_type_matrix::MatrixValueType::I8 => "matrix<i8>".to_string(),
                field_type_matrix::MatrixValueType::Unspecified => "matrix<_>".to_string(),
            },
        }
    }
}
