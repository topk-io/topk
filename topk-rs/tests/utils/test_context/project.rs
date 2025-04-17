use test_context::AsyncTestContext;
use topk_rs::{Client, ClientConfig};
use uuid::Uuid;

pub struct ProjectTestContext {
    pub client: Client,
    pub scope: String,
}

impl ProjectTestContext {
    pub fn wrap(&self, name: &str) -> String {
        format!("{}-{}", self.scope, name)
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

        Self { client, scope }
    }

    async fn teardown(self) {
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
}
