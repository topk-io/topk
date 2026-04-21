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

    let upsert = client
        .dataset("my-docs")
        .upsert_file(
            "doc-1",
            "/path/to/document.pdf",
            [("source", Value::string("internal"))],
        )
        .await?;

    client
        .dataset("my-docs")
        .wait_for_handle(&upsert.handle, None)
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
