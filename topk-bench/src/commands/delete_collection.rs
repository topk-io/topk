use clap::Parser;
use tracing::info;

use crate::commands::ProviderArg;
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::ProviderLike;

#[derive(Parser, Debug, Clone)]
pub struct DeleteCollectionArgs {
    #[arg(long, help = "Target collection")]
    pub(crate) collection: String,

    #[arg(short, long, help = "Target collection")]
    pub(crate) provider: ProviderArg,
}

pub async fn run(args: DeleteCollectionArgs) -> anyhow::Result<()> {
    // Create provider
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new().await?,
        ProviderArg::TopkPy => TopkPyProvider::new().await?,
        ProviderArg::TpufPy => TpufPyProvider::new().await?,
    };

    // Delete collection
    info!("Deleting collection");
    provider.delete_collection(args.collection).await?;

    Ok(())
}
