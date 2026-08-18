mod ddl;
mod discover;
mod error;
mod sink;
pub mod source;
mod spec;

pub const ID: &str = "_id";
pub const ID_PLACEHOLDER: &str = "<column>";

pub use ddl::absent;
pub use discover::discover;
pub use error::Error;
pub use sink::{document, documents, load, Outcome};
pub use source::uri::Uri;
pub use source::{connect, Duckdb, Source};
pub use spec::{Field, Index, Spec, Target, Type};
