mod ddl;
mod error;
mod preview;
mod sink;
pub mod source;
mod spec;
mod state;
mod value;

pub const ID: &str = "_id";
pub const ID_PLACEHOLDER: &str = "<column>";

pub use ddl::{absent, create};
pub use error::Error;
pub use preview::preview;
pub use sink::{documents, Outcome, Sink};
pub use source::{File, Scan, Source, Uri};
pub use spec::{discover, render, Field, Index, Spec, Target, Type};
pub use state::{Cursor, State};
pub use value::document;
