use std::{cell::RefCell, collections::HashSet};

use futures::future::FutureExt;
use futures::stream::{FuturesUnordered, StreamExt};
use test_context::AsyncTestContext;
use uuid::Uuid;

use topk_rs::{Client, ClientConfig, Error};

pub struct ProjectTestContext {
    pub client: Client,
    pub scope: String,
    pub used: RefCell<HashSet<String>>,
}

impl ProjectTestContext {
    pub fn wrap(&self, name: &str) -> String {
        let wrapped = format!("{}-{}", self.scope, name);
        self.used.borrow_mut().insert(wrapped.clone());
        wrapped
    }
}

impl AsyncTestContext for ProjectTestContext {
    async fn setup() -> Self {
        let host = std::env::var("TOPK_HOST").expect("TOPK_HOST not set");
        let region = std::env::var("TOPK_REGION").expect("TOPK_REGION not set");
        let api_key = std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY not set");
        let https = std::env::var("TOPK_HTTPS").unwrap_or("true".to_string()) == "true";

        let client = Client::new(
            ClientConfig::new(api_key, region)
                .with_host(host)
                .with_https(https),
        );

        Self {
            client,
            scope: format!("topk-rs-{}", Uuid::new_v4()),
            used: RefCell::new(HashSet::new()),
        }
    }

    async fn teardown(self) {
        let datasets = self.client.datasets();
        let collections = self.client.collections();
        let names = self.used.borrow().clone();

        let mut futs = FuturesUnordered::new();
        for name in names {
            futs.push(datasets.delete(name.clone()).boxed());
            futs.push(collections.delete(name).boxed());
        }

        while let Some(result) = futs.next().await {
            match result {
                Ok(_) => {}
                Err(Error::DatasetNotFound) | Err(Error::CollectionNotFound) => {}
                Err(e) => println!("Teardown error: {e:?}"),
            }
        }
    }
}
