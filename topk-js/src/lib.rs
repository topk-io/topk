//! # TopK JavaScript SDK
//!
//! This crate provides a JavaScript/TypeScript SDK for the TopK vector database service.
//! It exposes Rust functionality through NAPI bindings, allowing seamless integration
//! with Node.js applications.
//!
//! ## Key Features
//!
//! - **Client Management**: Create and configure TopK clients with authentication and retry settings
//! - **Collection Operations**: Create, list, get, and delete collections
//! - **Document Operations**: Insert, update, query, and delete documents
//! - **Query Building**: Rich query API with filtering, selection, and sorting
//! - **Schema Definition**: Type-safe schema definition for collections
//! - **Data Types**: Support for various data types including vectors, sparse vectors, and lists
//!
//! ## Example Usage
//!
//! ```javascript
//! const { Client, schema, query, data } = require('@topk-js/sdk');
//!
//! // Create a client
//! const client = new Client({
//!   api_key: 'your-api-key',
//!   region: 'us-east-1'
//! });
//!
//! // Create a collection
//! const collection = await client.collections().create('my-collection', {
//!   title: schema.text().required(),
//!   content: schema.text(),
//!   embedding: schema.f32_vector({ dimension: 384 }).index(
//!     schema.vector_index({ metric: 'cosine' })
//!   )
//! });
//!
//! // Insert documents
//! await client.collection('my-collection').upsert([
//!   {
//!     id: 'doc1',
//!     title: 'Hello World',
//!     content: 'This is a test document',
//!     embedding: data.f32_vector([0.1, 0.2, 0.3, ...])
//!   }
//! ]);
//!
//! // Query documents
//! const results = await client.collection('my-collection').query(
//!   query.select({
//!     title: query.field('title'),
//!     score: query.field('score')
//!   }).filter(
//!     query.field('title').contains('Hello')
//!   ).topk(query.field('score'), 10)
//! );
//! ```

mod utils;

mod client;
mod data;
mod error;
mod expr;
mod query;
mod schema;

#[macro_export]
macro_rules! try_cast_ref {
    ($env:expr, $obj:expr, $type:ty) => {{
        let obj = Unknown::from_napi_value($env, $obj)?;

        let env = napi::Env::from_raw($env);
        let is_instance = <$type>::instance_of(env, &obj)?;

        if is_instance {
            <$type>::from_napi_ref($env, $obj)
        } else {
            Err(napi::Error::from_reason("Invalid type"))
        }
    }};
}
