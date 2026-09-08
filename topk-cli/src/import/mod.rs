use std::sync::OnceLock;

use indicatif::MultiProgress;

mod coerce;
mod ddl;
mod decode;
mod error;
mod preview;
mod sink;
pub mod source;
mod spec;
mod state;

pub const ID: &str = "_id";
pub const ID_PLACEHOLDER: &str = "<column>";

static PROGRESS: OnceLock<MultiProgress> = OnceLock::new();

pub fn set_progress(progress: MultiProgress) {
    let _ = PROGRESS.set(progress);
}

pub fn note(message: String) {
    match PROGRESS.get() {
        Some(progress) => {
            let _ = progress.println(message);
        }
        None => eprintln!("{message}"),
    }
}

pub use ddl::{absent, create};
pub use error::Error;
pub use preview::{documents, preview};
pub use sink::{build_document, LoadOutcome, Sink};
pub use source::{Cursor, Source, Table, Uri};
pub use spec::{bind_columns, discover, render, Field, Spec, Target, Type};
pub use state::{Mark, State};
