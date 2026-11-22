mod coerce;
mod ddl;
mod document;
mod error;
mod execution;
mod expr;
pub mod plan;
pub mod source;
pub mod state;

pub const ID: &str = "_id";
pub const SEMANTIC_TEXT_LIMIT: usize = 32768;

pub use coerce::floats;
pub use error::Error;
pub use execution::{Aimd, Event, Import, Outcome, ReadOptions, WriteOptions};
pub use expr::Expr;
