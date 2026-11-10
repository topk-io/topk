// One test target, not seven: every top-level tests/*.rs is its own crate and
// links bundled duckdb all over again.
#[macro_use]
mod common;

mod cli;
mod document;
mod plan;
mod resume;
mod sources;
mod spec;
mod uri;
