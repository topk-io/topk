pub mod cache;
pub mod client;
pub mod commands;
pub mod config;
pub mod datasets;
pub mod output;
pub mod util;

#[cfg(test)]
pub mod test_context {
    use assert_cmd::Command;
    use test_context::AsyncTestContext;
    use topk_rs::{Client, ClientConfig};
    use uuid::Uuid;

    use crate::commands::dataset::CreateDatasetResult;

    pub struct CliTestContext {
        pub client: Client,
        pub scope: String,
        pub region: String,
    }

    impl CliTestContext {
        pub fn wrap(&self, name: &str) -> String {
            format!("{}-{}", self.scope, name)
        }

        pub fn create_dataset(&self, name: &str) -> CreateDatasetResult {
            let out = Command::cargo_bin("topk")
                .unwrap()
                .args([
                    "-o",
                    "json",
                    "dataset",
                    "create",
                    "--region",
                    &self.region,
                    name,
                ])
                .output()
                .unwrap();

            let result: CreateDatasetResult = serde_json::from_slice(&out.stdout).unwrap();

            assert_eq!(result.dataset.name, name);

            result
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

            let host = std::env::var("TOPK_HOST").expect("TOPK_HOST not set");
            let region = std::env::var("TOPK_REGION").expect("TOPK_REGION not set");
            let api_key = std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY not set");
            let https =
                std::env::var("TOPK_HTTPS").unwrap_or_else(|_| "true".to_string()) == "true";

            let client = Client::new(
                ClientConfig::new(api_key, region.clone())
                    .with_host(host)
                    .with_https(https),
            );

            Self {
                client,
                scope,
                region,
            }
        }

        async fn teardown(self) {
            self.cleanup_datasets().await;
        }
    }
}
