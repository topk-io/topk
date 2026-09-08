// One test target, not seven: every top-level tests/*.rs is its own crate and
// links bundled duckdb all over again.
mod common;

mod catalog;
mod coerce;
mod discover;
mod e2e;
mod plan;
mod sources;
mod spec;
mod state;
