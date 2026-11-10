use std::sync::OnceLock;

use indicatif::MultiProgress;

mod coerce;
mod ddl;
mod decode;
mod error;
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
        Some(progress) if !progress.is_hidden() => {
            let _ = progress.println(message);
        }
        _ => eprintln!("{message}"),
    }
}

pub use error::Error;
pub use sink::{build_row, documents, Import, LoadOutcome, WriteOptions};
pub use source::{Cursor, File, Source, Table, Uri};
pub use spec::{discover, Element, Field, Index, PlanOptions, Spec, Target, Type};
pub use state::{Checkpoint, Destination, Mark, Run, RunId, State};
