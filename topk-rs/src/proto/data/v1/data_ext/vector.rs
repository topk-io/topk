use crate::proto::data::v1::{vector, Vector};

impl Vector {
    pub fn f32(values: Vec<f32>) -> Self {
        Vector {
            vector: Some(vector::Vector::Float(vector::Float { values })),
        }
    }

    pub fn u8(values: Vec<u8>) -> Self {
        Vector {
            vector: Some(vector::Vector::Byte(vector::Byte { values })),
        }
    }

    pub fn len(&self) -> Option<usize> {
        match &self.vector {
            Some(vector::Vector::Float(vector::Float { values })) => Some(values.len()),
            Some(vector::Vector::Byte(vector::Byte { values })) => Some(values.len()),
            _ => None,
        }
    }
}
