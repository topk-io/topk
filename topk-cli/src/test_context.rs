use test_context::AsyncTestContext;
use topk_rs::{Client, ClientConfig};
use uuid::Uuid;

pub struct CliTestContext {
    pub client: Client,
    pub scope: String,
}

impl CliTestContext {
    pub fn wrap(&self, name: &str) -> String {
        format!("{}-{}", self.scope, name)
    }

    async fn cleanup_datasets(&self) {
        let client = self.client.datasets();
        let response = match client.list().await {
            Ok(r) => r,
            Err(e) => {
                println!("Failed to list datasets on teardown: {}", e);
                return;
            }
        };
        for dataset in &response.datasets {
            if dataset.name.starts_with(&self.scope) {
                println!("Deleting dataset: {}", dataset.name);
                if let Err(e) = client.delete(&dataset.name).await {
                    println!("Failed to delete dataset {}: {}", dataset.name, e);
                }
            }
        }
    }
}

impl AsyncTestContext for CliTestContext {
    async fn setup() -> Self {
        let scope = format!("topk-cli-{}", Uuid::new_v4().simple());

        let host = std::env::var("TOPK_HOST").unwrap_or("topk.dev".to_string());
        let region = std::env::var("TOPK_REGION").unwrap_or("sunflower".to_string());
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
        self.cleanup_datasets().await;
    }
}
