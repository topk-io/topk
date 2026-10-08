<p align="center" style="padding: 40px 0;">
   <img src="./assets/topk-logo-light.svg#gh-light-mode-only">
   <img src="./assets/topk-logo-dark.svg#gh-dark-mode-only">
</p>

# TopK

[TopK](https://topk.io) is a unified retrieval engine for hybrid search, vector search, multi-vector retrieval, keyword search (BM25) custom ranking, and managed inference in one API. Built on object storage for 10x lower cost and massive scale.

## What TopK Does

**Retrieval Engine** — Store and query dense vectors, sparse vectors, multi-vector embeddings, and metadata in a single collection. Filter by metadata, rank with custom expressions, and run managed inference — all in a single API.

## Docs

Start with the [documentation](https://docs.topk.io) for quickstart guides, API reference, and product documentation.

## Building with a coding agent

Using Claude Code, Cursor, Codex, or another coding agent? Install the [TopK skill](./skills/topk/) so the agent writes correct TopK code. It covers schema design, chunking, hybrid queries, multi-tenancy, and the common mistakes.

```sh
npx skills add topk-io/topk --skill topk
```

## SDKs

- [Python SDK](./topk-py/) - Python SDK for TopK API
- [JavaScript SDK](./topk-js/) - Javascript SDK for TopK API with full Typescript support
- [Rust SDK](./topk-rs/) - Rust SDK for TopK API
- [SQL](./topk-sql/) - Connect any PostgreSQL client to TopK

## CLI

- [TopK CLI](./topk-cli/) - CLI for TopK API
