pub mod es;
pub mod minio;
pub mod mongo;
pub mod mysql;
pub mod parquet;
pub mod pg;
pub mod sql;
pub mod sqlite;
pub mod xlsx;

use async_trait::async_trait;
use topk::import::Target;
use topk_rs::proto::v1::data::Document;

#[async_trait(?Send)]
pub trait Seed {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target>;

    /// Where to connect; `None` when `from` is the whole locator, as for files.
    fn url(&self) -> Option<String> {
        None
    }
}

/// Seeded targets carry the columns discovery would have found: the spec is a
/// whitelist, so a target without fields imports nothing but ids.
pub async fn discovered(target: Target, url: Option<String>) -> anyhow::Result<Target> {
    let uri: topk::import::Uri = match url {
        Some(url) => url.parse()?,
        None => target.from.parse()?,
    };
    let source = topk::import::Source::connect(&uri, &topk::endpoint::Endpoint::default()).await?;
    let spec =
        topk::import::discover(&source, std::slice::from_ref(&target.from), None, None).await?;
    let found = spec
        .collections
        .into_iter()
        .find(|(_, found)| found.from == target.from);
    Ok(match found {
        Some((_, found)) => Target {
            fields: found.fields,
            id: target.id.clone().or(found.id),
            ..target
        },
        None => target,
    })
}
