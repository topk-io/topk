use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Deserialize;
use topk_rs::{
    doc,
    proto::v1::{
        control::{FieldSpec, VectorDistanceMetric},
        data::{stage::select_stage::SelectExpr, Document},
    },
    query::select,
    Client, ClientConfig,
};
use tracing::info;

use crate::{
    config::LoadConfig,
    providers::{Provider, ProviderLike},
};

#[derive(Clone)]
pub struct TopkRsProvider {
    /// Topk-rs client
    client: Client,

    /// Collection name
    collection: String,
}

#[derive(Deserialize)]
pub struct TopkRsSettings {
    /// Topk API key
    pub topk_api_key: String,
    /// Topk region
    pub topk_region: String,
    /// Topk host
    pub topk_host: Option<String>,
    /// Topk HTTPS
    pub topk_https: Option<bool>,
}

impl std::fmt::Debug for TopkRsSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TopkRsSettings {{ topk_api_key: ********, topk_region: {}, topk_host: {:?}, topk_https: {:?} }}",
            self.topk_region, self.topk_host, self.topk_https
        )
    }
}

/// Creates a new TopkRsProvider.
pub async fn new(collection: String) -> anyhow::Result<Provider> {
    let settings = TopkRsSettings::load_config()?;
    info!(?settings, "Creating TopkRsProvider");

    let client = Client::new(
        ClientConfig::new(settings.topk_api_key, settings.topk_region)
            .with_host(settings.topk_host.unwrap_or("topk.io".into()))
            .with_https(settings.topk_https.unwrap_or(true)),
    );

    Ok(Provider::TopkRs(TopkRsProvider { client, collection }))
}

#[async_trait]
impl ProviderLike for TopkRsProvider {
    async fn setup(&self) -> anyhow::Result<()> {
        match self.client.collections().get(&self.collection).await {
            Ok(_) => Ok(()),
            Err(topk_rs::Error::CollectionNotFound) => {
                // Create collection
                let collection = self
                    .client
                    .collections()
                    .create(
                        &self.collection,
                        HashMap::from_iter([(
                            "vector".to_string(),
                            FieldSpec::f32_vector(768, true, VectorDistanceMetric::Cosine),
                        )]),
                    )
                    .await?;

                info!(?collection, "Created new collection");

                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn ping(&self) -> anyhow::Result<Duration> {
        let start = Instant::now();

        match self
            .client
            .collection("non-existing-collection")
            .query(select(Vec::<(&str, SelectExpr)>::new()), None, None)
            .await
        {
            Ok(_) => anyhow::bail!("query should have failed"),
            Err(e) => match e {
                topk_rs::Error::CollectionNotFound => {
                    // Expected error
                }
                _ => return Err(e.into()),
            },
        };

        Ok(start.elapsed())
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        let batch = batch
            .into_iter()
            .map(|mut doc| {
                if let Some(id_val) = doc.fields.remove("id") {
                    doc.fields.insert("_id".to_string(), id_val);
                }
                doc.into()
            })
            .collect();

        self.client
            .collection(&self.collection)
            .upsert(batch)
            .await?;

        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
