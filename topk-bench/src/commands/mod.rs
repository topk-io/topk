use clap::ValueEnum;

pub mod ingest;
pub mod query;

pub const BUCKET_NAME: &str = "jergu-test";

#[derive(ValueEnum, Clone, Debug)]
pub enum ProviderArg {
    TopkRs,
    TopkPy,
    TpufPy,
}
