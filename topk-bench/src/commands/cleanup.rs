use clap::Parser;
use tracing::info;

use crate::providers::{new_provider, ProviderArg, ProviderLike};

#[derive(Parser, Debug, Clone)]
pub struct CleanupArgs {
    #[arg(short, long, help = "Target provider")]
    pub(crate) provider: ProviderArg,

    #[arg(short, long, help = "Wet run")]
    pub(crate) wet: bool,
}

pub async fn run(args: CleanupArgs) -> anyhow::Result<()> {
    // Create provider
    let provider = new_provider(&args.provider).await?;

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
