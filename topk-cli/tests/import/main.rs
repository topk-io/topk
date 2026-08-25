// One test target, not seven: every top-level tests/*.rs is its own crate and
// links bundled duckdb all over again.
mod common;

mod catalog;
mod coerce;
mod discover;
mod dry_run;
mod e2e;
mod sources;
mod spec;
