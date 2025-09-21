use pyo3::prelude::*;

mod client;
mod data;
mod error;
mod expr;
mod query;
mod schema;

#[macro_export]
macro_rules! module {
    ($m:ident, $name:expr, $module:ty) => {{
        let module = pyo3::wrap_pymodule!($module);

        // add to parent module
        $m.add_wrapped(module)?;

        // Fix for https://github.com/PyO3/pyo3/issues/759.
        // We need to register the module in sys.modules to make it available
        // to the Python interpreter. PYO3 does not do this automatically.
        $m.py()
            .import("sys")?
            .getattr("modules")?
            .set_item(format!("topk_sdk.{}", $name), module($m.py()))
    }};
}

////////////////////////////////////////////////////////////
/// TopK SDK
///
/// This is the main module that contains all the functionality of the TopK SDK.
/// It is the entry point for the Python interpreter.
////////////////////////////////////////////////////////////

#[pymodule]
#[pyo3(name = "topk_sdk")]
pub fn topk_sdk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // sub modules
    module!(m, "query", query::pymodule)?;
    module!(m, "schema", schema::pymodule)?;
    module!(m, "data", data::pymodule)?;
    module!(m, "error", error::pymodule)?;

    // clients
    m.add_class::<client::sync::Client>()?;
    m.add_class::<client::sync::CollectionsClient>()?;
    m.add_class::<client::sync::CollectionClient>()?;

    m.add_class::<client::r#async::AsyncClient>()?;
    m.add_class::<client::r#async::AsyncCollectionsClient>()?;
    m.add_class::<client::r#async::AsyncCollectionClient>()?;

    // classes
    m.add_class::<data::collection::Collection>()?;

    Ok(())
}
