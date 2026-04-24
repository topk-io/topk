pub mod ask;
pub mod dataset;
pub mod delete;
pub mod list;
pub mod login;
pub mod search;
pub mod upload;

#[cfg(test)]
pub mod test_context {
    use assert_cmd::Command;
    use serde::de::DeserializeOwned;
    use test_context::AsyncTestContext;
    use topk_rs::Client;
    use uuid::Uuid;

    use crate::{client::make_global_client, commands::dataset::CreateDatasetResult};

    pub trait OutputJsonExt {
        fn json<T: DeserializeOwned>(&self) -> serde_json::Result<T>;
    }

    impl OutputJsonExt for std::process::Output {
        fn json<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
            serde_json::from_slice(&self.stdout)
        }
    }

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

            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let result: CreateDatasetResult = out.json().unwrap();

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

            let client = make_global_client(&api_key, &host, https);

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
