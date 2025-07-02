include!(concat!(env!("OUT_DIR"), "/topk.data.v1.rs"));

mod data_ext;
mod query_ext;
pub use query_ext::expr_ext::function::QueryVector;
