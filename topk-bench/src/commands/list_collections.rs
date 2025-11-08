use clap::Parser;

use crate::commands::ProviderArg;
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::ProviderLike;

#[derive(Parser, Debug, Clone)]
pub struct ListCollectionsArgs {
    #[arg(short, long, help = "Target provider")]
    pub(crate) provider: ProviderArg,
}

pub async fn run(args: ListCollectionsArgs) -> anyhow::Result<()> {
    // Create provider
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new().await?,
        ProviderArg::TopkPy => TopkPyProvider::new().await?,
        ProviderArg::TpufPy => TpufPyProvider::new().await?,
    };

    let collections = provider.list_collections().await?;
    for collection in collections {
        println!("{}", collection);
    }

    Ok(())
}
