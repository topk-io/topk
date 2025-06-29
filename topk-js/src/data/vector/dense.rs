use napi_derive::napi;

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct Vector(pub(crate) VectorUnion);

impl Vector {
    pub(crate) fn float(values: Vec<f32>) -> Self {
        Vector(VectorUnion::Float { values })
    }

    pub(crate) fn byte(values: Vec<u8>) -> Self {
        Vector(VectorUnion::Byte { values })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VectorUnion {
    Float { values: Vec<f32> },
    Byte { values: Vec<u8> },
}

impl Into<topk_rs::data::Vector> for Vector {
    fn into(self) -> topk_rs::data::Vector {
        match self.0 {
            VectorUnion::Float { values } => {
                topk_rs::data::Vector::F32(values.iter().map(|v| *v as f32).collect())
            }
            VectorUnion::Byte { values } => topk_rs::data::Vector::U8(values),
        }
    }
}

impl Into<topk_rs::proto::v1::data::Vector> for Vector {
    fn into(self) -> topk_rs::proto::v1::data::Vector {
        topk_rs::proto::v1::data::Vector {
            vector: Some(match self.0 {
                VectorUnion::Float { values } => topk_rs::proto::v1::data::vector::Vector::Float(
                    topk_rs::proto::v1::data::vector::Float {
                        values: values.iter().map(|v| *v as f32).collect(),
                    },
                ),
                VectorUnion::Byte { values } => topk_rs::proto::v1::data::vector::Vector::Byte(
                    topk_rs::proto::v1::data::vector::Byte { values },
                ),
            }),
        }
    }
}
