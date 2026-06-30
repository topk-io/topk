# TopK Rust SDK

The TopK Rust library provides convenient access to the TopK API from Rust applications.

## Documentation

The full documentation can be found at [docs.topk.io](https://docs.topk.io).

## Installation

```sh
cargo add topk-rs
cargo add tokio --features macros,rt-multi-thread
```

If you want to consume streamed ask responses as shown below:

```sh
cargo add futures-util
```

## Prerequisites

- **API key** — sign in to [console.topk.io](https://console.topk.io) and generate an API key.
- **Region** — available regions are listed at [docs.topk.io/regions](https://docs.topk.io/regions).

## Usage

### Hybrid Search

```rust
use topk_rs::{
    doc, schema,
    proto::v1::control::{FieldIndex, FieldSpec, KeywordIndexType},
    query::{field, fns, select},
    Client, ClientConfig, Error,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::new(ClientConfig::new(
        std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY is not set"),
        "aws-us-east-1-elastica",
    ));

    // Create a collection
    client
        .collections()
        .create(
            "books",
            schema!(
                "title" => FieldSpec::text(true).with_index(FieldIndex::keyword(KeywordIndexType::Text)),
                "content" => FieldSpec::text(false).with_index(FieldIndex::semantic()),
            ),
            None,
        )
        .await?;

    // Upsert documents
    client
        .collection("books")
        .upsert(vec![
            doc!(
                "_id" => "1",
                "title" => "Catcher in the Rye",
                "content" => "IF YOU REALLY WANT TO HEAR about it, the first thing you'll probably want to know is ...",
                "author" => "J.D. Salinger",
                "rating" => 3.8f64
            ),
            doc!(
                "_id" => "2",
                "title" => "1984",
                "content" => "It was a bright cold day in April, and the clocks were striking thirteen. Winston Smith, ...",
                "author" => "George Orwell",
                "rating" => 4.7f64
            ),
        ])
        .await?;

    // Query with hybrid search
    let results = client
        .collection("books")
        .query(
            select([
                // Select document fields to return
                ("title", field("title")),
                ("author", field("author")),
                // Compute semantic similarity of content field with the query
                ("similarity_score", fns::semantic_similarity("content", "What is the meaning of life?")),
            ])
            // Filter documents by metadata
            .filter(field("rating").gte(3.0f64))
            // Rank using the computed similarity score and rating
            .sort(field("rating").mul(field("similarity_score")), false)
            // Get top 10 highest ranked documents
            .limit(10),
            None,
            None,
        )
        .await?;

    for doc in results {
        println!("{:?}", doc);
    }

    Ok(())
}
```

### Vector Search

```rust
use topk_rs::{
    doc, schema,
    proto::v1::control::{FieldIndex, FieldSpec, VectorDistanceMetric},
    query::{field, fns, select},
    Client, ClientConfig, Error,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::new(ClientConfig::new(
        std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY is not set"),
        "aws-us-east-1-elastica",
    ));

    // Create a collection with a vector field (dimension must match your embedding model's output size)
    client
        .collections()
        .create(
            "books",
            schema!(
                "title" => FieldSpec::text(true, None),
                "embedding" => FieldSpec::f32_vector(1536, true).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)),
            ),
            None,
        )
        .await?;

    // Upsert documents with embeddings
    client
        .collection("books")
        .upsert(vec![
            doc!("_id" => "1", "title" => "Catcher in the Rye", "embedding" => vec![0.1f32, 0.2, /* ... */]),
            doc!("_id" => "2", "title" => "1984",               "embedding" => vec![0.9f32, 0.8, /* ... */]),
        ])
        .await?;

    // Query the nearest neighbors to a query vector
    let results = client
        .collection("books")
        .query(
            select([
                ("title", field("title")),
                ("distance", fns::vector_distance("embedding", vec![0.8f32, 0.9, /* ... */])),
            ])
            // Return the 10 closest documents (ascending = closest first)
            .topk(field("distance"), 10, true),
            None,
            None,
        )
        .await?;

    for doc in results {
        println!("{:?}", doc);
    }

    Ok(())
}
```

### File Search

```rust
use futures_util::StreamExt;
use topk_rs::proto::v1::ctx::ask_result::Message;
use topk_rs::proto::v1::data::Value;
use topk_rs::{Client, ClientConfig, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::new(ClientConfig::new(
        std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY is not set"),
        "aws-us-east-1-elastica",
    ));

    client.datasets().create("my-docs").await?;

    let handle = client
        .dataset("my-docs")
        .upsert_file(
            "doc-1",                              // document ID
            "/path/to/document.pdf",              // path to file
            [
                ("kind", Value::string("report")),
                ("department", Value::string("finance")),
            ],
        )
        .await?;

    client
        .dataset("my-docs")
        .wait_for_handle(&handle, None)
        .await?;

    let mut stream = client
        .ask("What was the total net income of Bank of America in 2024?", ["my-docs"], None, None, None)
        .await?;

    while let Some(message) = stream.next().await {
        let message = message?;

        if let Some(Message::Answer(answer)) = message.message {
            println!("{:#?}", answer.facts);
        }
    }

    Ok(())
}
```

## Handling errors

```rust
use topk_rs::{Client, ClientConfig, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::new(ClientConfig::new(
        std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY is not set"),
        "aws-us-east-1-elastica",
    ));

    match client.ask("What was the total net income of Bank of America in 2024?", ["my-docs"], None, None, None).await {
        Ok(_) => {}
        Err(Error::DatasetNotFound) => eprintln!("Dataset does not exist"),
        Err(Error::PermissionDenied | Error::Unauthenticated(_)) => {
            eprintln!("Check your API key")
        }
        Err(Error::QuotaExceeded(_)) => eprintln!("Usage quota exceeded"),
        Err(Error::SlowDown(_)) => eprintln!("Rate limited; the client will retry automatically"),
        Err(err) => eprintln!("Unexpected error: {err}"),
    }

    Ok(())
}
```

| Error | Description |
| --- | --- |
| `CollectionNotFound` | Collection does not exist |
| `CollectionAlreadyExists` | Collection with this name already exists |
| `CollectionValidationError` | Invalid collection name or schema |
| `DatasetNotFound` | Dataset does not exist |
| `DatasetAlreadyExists` | Dataset with this name already exists |
| `DocumentValidationError` | Invalid document |
| `SchemaValidationError` | Invalid schema |
| `PermissionDenied` | Invalid or missing API key |
| `Unauthenticated` | Authentication failed |
| `QuotaExceeded` | Usage quota exceeded |
| `RequestTooLarge` | Request payload too large |
| `SlowDown` | Rate limited by the server (retried automatically) |
| `QueryLsnTimeout` | Timed out waiting for write consistency |
| `RetryTimeout` | Retry chain or wait-for-handle polling timed out |

### Retries

The client automatically retries on `SlowDown`, transport errors, and LSN consistency timeouts. Retry behaviour can be configured via `RetryConfig`:

```rust
use std::time::Duration;
use topk_rs::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

let client = Client::new(
    ClientConfig::new(
        std::env::var("TOPK_API_KEY").expect("TOPK_API_KEY is not set"),
        "aws-us-east-1-elastica",
    )
    .with_retry_config(RetryConfig {
        max_retries: 5,
        timeout: Duration::from_secs(60),
        backoff: BackoffConfig {
            init_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(5),
            base: 2,
        },
    }),
);
```

## Requirements

A current stable Rust toolchain with Rust 2021 edition support, plus Tokio for async execution.
