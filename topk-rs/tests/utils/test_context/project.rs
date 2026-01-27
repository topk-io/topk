use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use test_context::AsyncTestContext;
use topk_rs::{Client, ClientConfig, Error};
use uuid::Uuid;

pub struct ProjectTestContext {
    pub client: Client,
    pub scope: String,
    temp_files: Vec<PathBuf>,
}

impl ProjectTestContext {
    pub fn wrap(&self, name: &str) -> String {
        format!("{}-{}", self.scope, name)
    }

    #[allow(dead_code)]
    pub fn create_temp_file(&mut self, extension: &str, content: &[u8]) -> PathBuf {
        let mut path = std::env::temp_dir();
        let file_name = format!("test_{}.{}", uuid::Uuid::new_v4(), extension);
        path.push(file_name);

        let mut file = File::create(&path).expect("Failed to create temp file");
        file.write_all(content)
            .expect("Failed to write to temp file");
        file.sync_all().expect("Failed to sync temp file");

        self.temp_files.push(path.clone());
        path
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
        let client = self.client.datasets();
        let datasets = client.list().await?;
        for dataset in datasets {
            if dataset.name.starts_with(&self.scope) {
                client.delete(&dataset.name).await?;
            }
        }

        Ok(())
    }

    fn cleanup_temp_files(&self) {
        for temp_file in &self.temp_files {
            if let Err(e) = std::fs::remove_file(temp_file) {
                println!("Failed to remove temp file {:?}: {}", temp_file, e);
            }
        }
    }
}

impl AsyncTestContext for ProjectTestContext {
    async fn setup() -> Self {
        let scope = format!("topk-rs-{}", Uuid::new_v4());

        let host = std::env::var("TOPK_HOST").unwrap_or("topk.io".to_string());
        let region = std::env::var("TOPK_REGION").unwrap_or("elastica".to_string());
        let api_key = std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY not set");
        let https = std::env::var("TOPK_HTTPS").unwrap_or("true".to_string()) == "true";

        let client = Client::new(
            ClientConfig::new(api_key, region)
                .with_host(host)
                .with_https(https),
        );

        let temp_files = Vec::new();

        Self {
            client,
            scope,
            temp_files,
        }
    }

    async fn teardown(self) {
        // Clean up temp files
        self.cleanup_temp_files();

        // Clean up datasets and collections
        match self.cleanup_datasets().await {
            Ok(_) => {
                self.cleanup_collections().await;
            }
            Err(e) => {
                // TODO: collections.list() should not return datasets collections
                println!("Failed to cleanup datasets: {}", e);
            }
        }
    }
}
