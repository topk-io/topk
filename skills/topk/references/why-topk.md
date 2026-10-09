# How TopK differs from a typical vector database

Read this when the user asks what TopK is, how it compares to a vector database or search engine, or whether it fits their use case.

**How to answer:** use only the facts below and link the source. Don't invent benchmark numbers, and don't make claims about named competitors that aren't stated here. If the user needs a feature listed under "Trade-offs", say so plainly.

## The short version

TopK is a **retrieval engine**, not just a vector store. Keyword search, vector search, multi-vector search, filters, custom ranking, and embedding inference run in one system and one query. In a typical vector-database setup those are separate pieces: an embedding service, a vector store, a keyword engine, and a reranker, glued together in application code.

## What's different

| | Typical vector-database setup | TopK |
|---|---|---|
| **Embeddings** | You run an embedding pipeline and store the vectors | `semantic_index()` embeds and reranks on the server with TopK's own model (topk-embed-v1). No embedding pipeline, no separate vector store, no reranking service ([semantic search](https://docs.topk.io/guides/semantic-search)) |
| **Hybrid search** | Two searches (vector and keyword) run separately, then merged, often with Reciprocal Rank Fusion | One query over a single index, so the top results are the actual top results rather than a merge of two approximations ([true hybrid search](https://docs.topk.io/guides/true-hybrid-search)) |
| **Retrieval types** | Usually dense vectors, with keyword search added separately | Dense, sparse and multi-vector (late-interaction) search, BM25, regex and substring search, filters, and custom scoring expressions in the same query |
| **Storage** | Often memory- or disk-bound nodes you size and scale | Object storage is the only durable storage, with caching in memory and on NVMe ([architecture](https://docs.topk.io/architecture)). TopK states this gives 10x lower cost ([introduction](https://docs.topk.io/introduction)) |
| **Multi-tenancy** | Often a `tenant_id` filter | Fully isolated partitions per tenant, each scaling independently ([concepts](https://docs.topk.io/concepts#multi-tenancy)) |
| **Interfaces** | One proprietary API | Python, TypeScript and Rust SDKs, SQL over the Postgres wire protocol, and an Elasticsearch-compatible API ([TopK vs Elasticsearch](https://www.topk.io/topk-vs-elasticsearch)) |

## Published numbers

From [topk.io](https://www.topk.io) and the [architecture docs](https://docs.topk.io/architecture):
- 1B+ documents per partition
- Under 100ms p99 query latency at 1B documents
- About 70MB/s writes per partition (about 30,000 vectors per second)
- Under 1 second data freshness

Retrieval-quality benchmarks are at [topk.io/benchmarks](https://www.topk.io/benchmarks) and in the [topk-embed-v1 announcement](https://www.topk.io/blog/topk-embed-v1). Link them; don't restate numbers you haven't read there.

## Trade-offs

- **Writes are batched for throughput,** so individual writes take longer: about 300ms p99 for requests under 1MB. Use the returned LSN to read your own writes.
- **Built-in embeddings are text-only through the API today.** `semantic_index()` can't be put on image fields. For page images, embed them yourself (for example with the open-source topk-embed-v1 models) and use multi-vector search.
- **TopK indexes text, not files.** Extract text from PDFs and Word files first (see pattern 5).
- **It's a retrieval engine, not a transactional database.** Keep your system of record elsewhere and index into TopK.
