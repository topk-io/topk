use std::time::{Duration, SystemTime, UNIX_EPOCH};

use test_context::AsyncTestContext;
use topk_rs::proto::v1::control::Dataset;
use topk_rs::{Client, ClientConfig, Error};
use uuid::Uuid;

const DATASET_CLEANUP_MAX_AGE: Duration = Duration::from_mins(60);

pub struct ProjectTestContext {
    pub client: Client,
    pub scope: String,
}

impl ProjectTestContext {
    pub fn wrap(&self, name: &str) -> String {
        format!("{}-{}", self.scope, name)
    }

    async fn cleanup_collections(&self) {
        let client = self.client.collections();
        let collections = client
            .list()
            .await
            .expect("Failed to list collections on teardown");
        for collection in collections {
            if collection.name.starts_with(&self.scope) {
                println!("Deleting collection: {}", collection.name);
                let res = client.delete(&collection.name).await;

                if let Err(e) = res {
                    println!("Failed to delete collection {}: {}", collection.name, e);
                }
            }
        }
    }

    async fn cleanup_datasets(&self) -> Result<(), Error> {
        let should_delete = |dataset: &Dataset| {
            // Skip if the name does not start with "topk-rs-"
            if !dataset.name.starts_with("topk-rs-") {
                return false;
            }
            // Current scope
            if dataset.name.starts_with(&self.scope) {
                return true;
            }
            let now = current_unix_timestamp();
            match parse_dataset_timestamp(&dataset.name) {
                Some(created_at) => {
                    let cutoff = now.saturating_sub(DATASET_CLEANUP_MAX_AGE.as_secs());
                    created_at < cutoff
                }
                None => true,
            }
        };

        let client = self.client.datasets();
        let datasets = client.list().await?;
        for dataset in datasets {
            if should_delete(&dataset) {
                println!("Deleting dataset: {}", dataset.name);
                if let Err(e) = client.delete(&dataset.name).await {
                    println!("Failed to delete dataset {}: {}", dataset.name, e);
                }
            }
        }
        Ok(())
    }
}

impl AsyncTestContext for ProjectTestContext {
    async fn setup() -> Self {
        let scope = format!("topk-rs-{}-{}", current_unix_timestamp(), Uuid::new_v4());

        let host = std::env::var("TOPK_HOST").unwrap_or("topk.io".to_string());
        let region = std::env::var("TOPK_REGION").unwrap_or("elastica".to_string());
        let api_key = std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY not set");
        let https = std::env::var("TOPK_HTTPS").unwrap_or("true".to_string()) == "true";

        let client = Client::new(
            ClientConfig::new(api_key, region)
                .with_host(host)
                .with_https(https),
        );

        Self { client, scope }
    }

    async fn teardown(self) {
        if let Err(e) = self.cleanup_datasets().await {
            println!("Failed to cleanup datasets: {}", e);
        }
        self.cleanup_collections().await;
    }
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs()
}

fn parse_dataset_timestamp(name: &str) -> Option<u64> {
    let parts: Vec<&str> = name.splitn(4, '-').collect();
    if parts.len() >= 3 && parts[0] == "topk" && parts[1] == "rs" {
        parts[2].parse().ok()
    } else {
        None
    }
}
