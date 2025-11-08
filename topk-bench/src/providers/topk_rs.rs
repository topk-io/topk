use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Deserialize;
use topk_rs::query::{field, filter, fns};
use topk_rs::{
    doc,
    proto::v1::{
        control::{FieldSpec, VectorDistanceMetric},
        data::stage::select_stage::SelectExpr,
    },
    query::select,
    Client, ClientConfig,
};
use tracing::info;

use crate::{
    config::LoadConfig,
    providers::{Provider, ProviderLike},
};
use crate::{data::Document, providers::Query};

#[derive(Deserialize)]
pub struct TopkRsSettings {
    /// Topk API key
    pub topk_api_key: String,

    /// Topk region
    pub topk_region: String,

    /// Topk host
    #[serde(default)]
    pub topk_host: Option<String>,

    /// Topk HTTPS
    #[serde(default)]
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

#[derive(Clone)]
pub struct TopkRsProvider {
    /// Topk-rs client
    client: Client,

    /// Collection name
    collection: String,
}

impl TopkRsProvider {
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
            .query(
                select(Vec::<(&str, SelectExpr)>::new()).limit(1),
                None,
                None,
            )
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

    async fn query_by_id(&self, id: String) -> anyhow::Result<Option<Document>> {
        let documents = self
            .client
            .collection(&self.collection)
            .query(filter(field("_id").eq(id.clone())).limit(1), None, None)
            .await?;

        match &documents[..] {
            [] => Ok(None),
            [document] => Ok(Some(document.clone().into())),
            _ => anyhow::bail!("multiple documents found for id: {}", id),
        }
    }

    async fn delete_by_id(&self, ids: Vec<String>) -> anyhow::Result<()> {
        self.client.collection(&self.collection).delete(ids).await?;

        Ok(())
    }

    async fn query(&self, query: Query) -> anyhow::Result<Vec<Document>> {
        let mut topk_query = select(vec![(
            "vector_distance",
            fns::vector_distance("dense_embedding", query.vector),
        )])
        .limit(query.top_k as u64);

        if let Some(numeric_selectivity) = query.numeric_selectivity {
            topk_query = topk_query.filter(field("numerical_filter").eq(numeric_selectivity));
        }
        if let Some(categorical_selectivity) = query.categorical_selectivity {
            topk_query = topk_query.filter(field("categorical_filter").eq(categorical_selectivity));
        }

        let documents = self
            .client
            .collection(&self.collection)
            .query(topk_query, None, None)
            .await?;

        Ok(documents.into_iter().map(|d| d.into()).collect())
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        let batch = batch
            .into_iter()
            .map(|mut doc| {
                if let Some(id_val) = doc.remove("id") {
                    doc.insert("_id", id_val);
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

    async fn list_collections(&self) -> anyhow::Result<Vec<String>> {
        let collections = self.client.collections().list().await?;

        Ok(collections.into_iter().map(|c| c.name).collect())
    }

    async fn delete_collection(&self, name: String) -> anyhow::Result<()> {
        self.client.collections().delete(name).await?;

        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
