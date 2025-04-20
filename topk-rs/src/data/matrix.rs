#[derive(Debug, Clone)]
pub enum Matrix {
    F32 { dimension: u32, values: Vec<f32> },
    U8 { dimension: u32, values: Vec<u8> },
}

impl Into<topk_protos::v1::data::Matrix> for Matrix {
    fn into(self) -> topk_protos::v1::data::Matrix {
        match self {
            Matrix::F32 { dimension, values } => {
                topk_protos::v1::data::Matrix::float(dimension, values)
            }
            Matrix::U8 { dimension, values } => {
                topk_protos::v1::data::Matrix::byte(dimension, values)
            }
        }
    }
}

impl From<topk_protos::v1::data::Matrix> for Matrix {
    fn from(matrix: topk_protos::v1::data::Matrix) -> Self {
        match matrix.matrix {
            Some(topk_protos::v1::data::matrix::Matrix::Float(values)) => Matrix::F32 {
                dimension: matrix.dimension,
                values: values.values,
            },
            Some(topk_protos::v1::data::matrix::Matrix::Byte(values)) => Matrix::U8 {
                dimension: matrix.dimension,
                values: values.values,
            },
            t => panic!("Invalid matrix type: {:?}", t),
        }
    }
}
