use pyo3::{create_exception, exceptions::PyException, prelude::*};

#[derive(Debug)]
pub struct RustError(pub topk_rs::Error);

impl From<RustError> for PyErr {
    fn from(value: RustError) -> Self {
        match value.0 {
            topk_rs::Error::CollectionNotFound => {
                CollectionNotFoundError::new_err(value.0.to_string())
            }
            topk_rs::Error::CollectionAlreadyExists => {
                CollectionAlreadyExistsError::new_err(value.0.to_string())
            }
            topk_rs::Error::SchemaValidationError(e) => {
                SchemaValidationError::new_err(format!("{:?}", e))
            }
            topk_rs::Error::DocumentValidationError(e) => {
                DocumentValidationError::new_err(format!("{:?}", e))
            }
            topk_rs::Error::InvalidArgument(e) => InvalidArgumentError::new_err(format!("{:?}", e)),
            _ => PyException::new_err(format!("topk returned error: {:?}", value.0)),
        }
    }
}

create_exception!(error, CollectionAlreadyExistsError, PyException);
create_exception!(error, CollectionNotFoundError, PyException);
create_exception!(error, SchemaValidationError, PyException);
create_exception!(error, DocumentValidationError, PyException);
create_exception!(error, InvalidArgumentError, PyException);

////////////////////////////////////////////////////////////
/// Error
///
/// This module contains the error definition for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "error")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "CollectionAlreadyExistsError",
        m.py().get_type::<CollectionAlreadyExistsError>(),
    )?;

    m.add(
        "CollectionNotFoundError",
        m.py().get_type::<CollectionNotFoundError>(),
    )?;

    m.add(
        "SchemaValidationError",
        m.py().get_type::<SchemaValidationError>(),
    )?;

    m.add(
        "DocumentValidationError",
        m.py().get_type::<DocumentValidationError>(),
    )?;

    m.add(
        "InvalidArgumentError",
        m.py().get_type::<InvalidArgumentError>(),
    )?;

    Ok(())
}
