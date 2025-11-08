use clap::Parser;
use tracing::info;

use crate::commands::ProviderArg;
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;

#[derive(Parser, Debug, Clone)]
pub struct ListCollectionsArgs {
    #[arg(short, long, help = "Target provider")]
    pub(crate) provider: ProviderArg,
}

pub async fn run(args: ListCollectionsArgs) -> anyhow::Result<()> {
    // Create provider
    info!(?args, "Creating provider");
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new().await?,
        ProviderArg::TopkPy => TopkPyProvider::new().await?,
        ProviderArg::TpufPy => TpufPyProvider::new().await?,
    };

    Ok(())
}
