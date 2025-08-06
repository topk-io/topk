use crate::proto::data::v1::{vector, Vector};

impl Vector {
    #[deprecated(note = "Use `list<f32>` instead")]
    pub fn f32(values: Vec<f32>) -> Self {
        Vector {
            #[allow(deprecated)]
            vector: Some(vector::Vector::Float(vector::Float { values })),
        }
    }

    #[deprecated(note = "Use `list<u8>` instead")]
    pub fn u8(values: Vec<u8>) -> Self {
        Vector {
            #[allow(deprecated)]
            vector: Some(vector::Vector::Byte(vector::Byte { values })),
        }
    }

    pub fn len(&self) -> Option<usize> {
        #[allow(deprecated)]
        match &self.vector {
            Some(vector::Vector::Float(vector::Float { values })) => Some(values.len()),
            Some(vector::Vector::Byte(vector::Byte { values })) => Some(values.len()),
            _ => None,
        }
    }
}
