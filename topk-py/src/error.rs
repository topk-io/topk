use pyo3::{create_exception, exceptions::PyException, prelude::*};

#[derive(Debug)]
pub struct RustError(pub topk_rs::Error);

impl From<RustError> for PyErr {
    fn from(value: RustError) -> Self {
        match value.0 {
            // Custom errors
            topk_rs::Error::QueryLsnTimeout => {
                QueryLsnTimeoutError::new_err(format!("{:?}", value.0))
            }
            // Not found errors
            topk_rs::Error::CollectionNotFound => {
                CollectionNotFoundError::new_err(value.0.to_string())
            }
            topk_rs::Error::CollectionAlreadyExists => {
                CollectionAlreadyExistsError::new_err(value.0.to_string())
            }
            // Validation errors
            topk_rs::Error::SchemaValidationError(e) => {
                SchemaValidationError::new_err(format!("{:?}", e))
            }
            topk_rs::Error::DocumentValidationError(e) => {
                DocumentValidationError::new_err(format!("{:?}", e))
            }
            topk_rs::Error::CollectionValidationError(e) => {
                CollectionValidationError::new_err(format!("{:?}", e))
            }
            topk_rs::Error::InvalidArgument(e) => InvalidArgumentError::new_err(e),
            // Request too large
            topk_rs::Error::RequestTooLarge(e) => RequestTooLargeError::new_err(e),
            topk_rs::Error::QuotaExceeded(e) => QuotaExceededError::new_err(e),
            topk_rs::Error::SlowDown(e) => SlowDownError::new_err(e),
            topk_rs::Error::PermissionDenied => PermissionDeniedError::new_err(value.0.to_string()),
            // Other errors
            _ => PyException::new_err(format!("topk returned error: {:?}", value.0)),
        }
    }
}

create_exception!(error, CollectionAlreadyExistsError, PyException);
create_exception!(error, CollectionNotFoundError, PyException);
create_exception!(error, SchemaValidationError, PyException);
create_exception!(error, DocumentValidationError, PyException);
create_exception!(error, CollectionValidationError, PyException);
create_exception!(error, InvalidArgumentError, PyException);
create_exception!(error, QueryLsnTimeoutError, PyException);
create_exception!(error, RequestTooLargeError, PyException);
create_exception!(error, QuotaExceededError, PyException);
create_exception!(error, SlowDownError, PyException);
create_exception!(error, PermissionDeniedError, PyException);

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

    m.add(
        "QueryLsnTimeoutError",
        m.py().get_type::<QueryLsnTimeoutError>(),
    )?;

    m.add(
        "CollectionValidationError",
        m.py().get_type::<CollectionValidationError>(),
    )?;

    Ok(())
}
