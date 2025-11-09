use clap::Parser;
use tracing::info;

use crate::commands::ProviderArg;
use crate::providers::chroma::ChromaProvider;
use crate::providers::topk_py::TopkPyProvider;
use crate::providers::topk_rs::TopkRsProvider;
use crate::providers::tpuf_py::TpufPyProvider;
use crate::providers::ProviderLike;

#[derive(Parser, Debug, Clone)]
pub struct CleanupArgs {
    #[arg(short, long, help = "Target provider")]
    pub(crate) provider: ProviderArg,

    #[arg(short, long, help = "Wet run")]
    pub(crate) wet: bool,
}

pub async fn run(args: CleanupArgs) -> anyhow::Result<()> {
    // Create provider
    let provider = match args.provider {
        ProviderArg::TopkRs => TopkRsProvider::new().await?,
        ProviderArg::TopkPy => TopkPyProvider::new().await?,
        ProviderArg::TpufPy => TpufPyProvider::new().await?,
        ProviderArg::Chroma => ChromaProvider::new().await?,
    };

    let collections = provider.list_collections().await?;

    for collection in &collections {
        println!("{}", collection);
    }

    if args.wet {
        for collection in collections {
            info!("Deleting collection {}", collection);
            provider.delete_collection(collection).await?;
        }
    }

    Ok(())
}
