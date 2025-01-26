use pyo3::{create_exception, exceptions::PyException, prelude::*};

create_exception!(error, CollectionNotFoundError, PyException);
create_exception!(error, SchemaValidationError, PyException);

////////////////////////////////////////////////////////////
/// Error
///
/// This module contains the error definition for the TopK SDK.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "error")]
pub fn pymodule(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "CollectionNotFoundError",
        m.py().get_type::<CollectionNotFoundError>(),
    )?;

    m.add(
        "SchemaValidationError",
        m.py().get_type::<SchemaValidationError>(),
    )?;

    Ok(())
}
