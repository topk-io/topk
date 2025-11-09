use clap::ValueEnum;

pub mod delete_collection;
pub mod ingest;
pub mod list_collections;
pub mod query;

pub const BUCKET_NAME: &str = "jergu-test";

#[derive(ValueEnum, Clone, Debug)]
pub enum ProviderArg {
    TopkRs,
    TopkPy,
    TpufPy,
    Chroma,
}
