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
pub use preview::preview;
pub use sink::{build_document, document_stream, LoadOutcome, Sink};
pub use source::{File, Scan, Source, Table, Uri};
pub use spec::{discover, render, validate_ids, Element, Field, Index, Spec, Target, Type};
pub use state::{Cursor, State};
