#[derive(Debug, Clone)]
pub enum Vector {
    F32(Vec<f32>),
    U8(Vec<u8>),
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

impl From<Vec<u8>> for Vector {
    fn from(values: Vec<u8>) -> Self {
        Vector::U8(values)
    }
}
