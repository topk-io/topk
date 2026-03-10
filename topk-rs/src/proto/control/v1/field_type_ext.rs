use super::field_type;
use crate::Error;

impl super::FieldType {
    pub fn data_type(&self) -> Result<&field_type::DataType, Error> {
        self.data_type.as_ref().ok_or(Error::InvalidProto)
    }
}
