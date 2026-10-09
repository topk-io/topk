---
name: topk
description: Builds search and retrieval on TopK, a hosted search database with built-in semantic search, BM25 keyword search, vector search, filtering, and multi-tenant partitions. Use when writing code with the topk-sdk Python package, the topk-js TypeScript package, or TopK SQL; when designing a TopK collection schema; or when indexing documents and implementing semantic, keyword, hybrid, vector, or per-tenant search with TopK.
license: MIT
metadata:
  author: topk-io
  version: "0.1.1"
---

# TopK

TopK stores documents in **collections** and searches them with one query builder: `select` → `filter` → `sort` → `limit`. Text fields with a `semantic_index()` are embedded and reranked on the server, so no embedding provider is needed.

## Start from a proven pattern

If the task matches one of these, start from the pattern in [references/patterns.md](references/patterns.md) and adapt it. Each one avoids mistakes that raise no error: truncated text, leaked tenant data, invented citations.

| Task | Pattern |
|---|---|
| Search docs, articles or other long text (chunking, semantic + BM25, one result per file) | 1. Search long documents (Python) |
| Keep each customer's data separate, including deleting a customer | 2. Per-customer partitions (Python) |
| Give an agent a search tool that answers with checked citations | 3. Agent retrieval tool (Python) |
| Product or catalog search with filters, price ranges and keyword boosts | 4. Catalog search (TypeScript) |

## Language references

Read the reference for the language you're writing. Each one has full, runnable patterns:

- **Python** (`topk-sdk`): [references/python.md](references/python.md)
- **TypeScript** (`topk-js`): [references/typescript.md](references/typescript.md)
- **SQL** (Postgres wire protocol): [references/sql.md](references/sql.md)

If something isn't covered here, read the docs as markdown: index at https://docs.topk.io/llms.txt, and any page by appending `.md` to its URL.

## Setup

Install `topk-sdk` (Python, imported as `topk_sdk`) or `topk-js` (TypeScript). Create the client from `TOPK_API_KEY` and `TOPK_REGION`. Never hardcode or print the key. The API key is scoped to one project, so no project ID is needed.

Regions: `aws-us-east-1-elastica`, `aws-eu-central-1-monstera`, `gcp-us-east4-aloe`.

To check credentials and look at existing data, run the bundled script. It needs only `topk-sdk` installed:

```bash
python scripts/topk_inspect.py check                # env vars, connectivity, list collections
python scripts/topk_inspect.py describe <collection> [--partition <name>]   # schema, count, 3 sample docs
```

## Design the schema

TopK is schemaless by default. Declare only the fields you index or want type-checked; other fields can be stored on any document.

Choose **one** index per field:

| Need | Index | Notes |
|---|---|---|
| Natural-language search over text | `semantic_index()` | Server-side embeddings and reranking. **Also supports BM25**, so one field covers hybrid search. Max **4,096 characters** per value. |
| Keyword-only (BM25) search over text | `keyword_index()` | No length cap beyond the document size. |
| Your own embeddings | `vector_index(metric=...)` on `f32_vector(dim)` and similar | Metrics: `cosine`, `euclidean`, `dot_product`, `hamming` (binary). |
| Late-interaction (ColBERT-style) | `multi_vector_index(metric="maxsim")` on `matrix(...)` | |
| Substring or regex filters | `ngram_index()` | Speeds up `contains` and `regexp_match`. |

A field holds **one** index. Chaining `.index(a).index(b)` silently keeps only `b`. To combine keyword and semantic search, use `semantic_index()` alone.

## Rules that cause most failures

1. **Chunk long text.** A value in a `semantic_index()` field longer than 4,096 characters fails with `DocumentValidationError ... TextTooLong { max_length: 4096 }`. Split long documents into passages of about 2,000–3,500 characters, one document per chunk. Store `source`, `chunk_index`, and the text on each chunk. Use smaller chunks for non-Latin scripts, because a document over 200KB *including its generated embeddings* fails with `DocumentTooLarge`. Don't truncate text to fit; truncated text can't be found by semantic search. If the user wants **one result per file**, still index chunks: query `limit(k * 4)`, keep the best-scoring chunk per `source`, and return the top `k` files.
2. **Always end a query with `.limit(k)`**, after `.sort(...)` when you rank by a score. Use `.count()` to count instead.
3. **`bm25_score()` requires a `match(...)` in the filter**, on a field with a keyword or semantic index. `match(q)` keeps only documents that contain at least one query term. When keyword terms should *boost* without excluding documents, use `should(term)`.
4. **Scores aren't normalized.** `semantic_similarity` and `bm25_score` have different, data-dependent ranges (for example, semantic about 1–4 and BM25 about 0.02–15). Before choosing hybrid weights, print a few real scores, or normalize them. Semantic search always returns its nearest neighbours, even when nothing is relevant. If "no results" is the right answer for an off-topic query, calibrate a minimum score from real queries and filter below it.
5. **Sort direction depends on the score.** Semantic similarity, BM25, and cosine or dot-product vector scores are higher-is-better: sort descending. Euclidean distance is lower-is-better: sort ascending.
6. **Don't delete and recreate a collection under the same name.** Writes right after a same-name recreate can be silently dropped. Use a new name (for example `docs-v2`), or keep the collection and delete documents instead.
7. **Treat "already exists" as success** when creating collections in setup scripts: catch `CollectionAlreadyExistsError` (Python), or check `err.message === "collection already exists"` (TypeScript).
8. **Batch writes.** One upsert request is at most 8MB, and one document at most 200KB. Upsert in batches of a few hundred documents.
9. **Read your own writes with the LSN.** `upsert`, `update`, and `delete` return an LSN string. Pass it as `lsn=` to `query`, `count`, or `get` so the read includes that write. Don't use `sleep`.
10. **Every document needs a string `_id`.** Upserting an existing `_id` replaces the whole document. To change some fields and keep the rest, use `update`. In Python `topk-sdk` 0.15, pass `fail_on_missing=False` explicitly, because the argument is required at runtime.
11. **Combine scores inside `.sort()`, not in a second `.select()`.** A later `select` that does arithmetic on score aliases (for example `select(score=field("sem") * 0.7 + ...)`) silently returns wrong values. Compute each score once in `select`, then sort by the weighted expression. To display the final score, recompute it in your code.

**TypeScript only:** whole-number values like `99` are sent as integers, which `float()` fields reject; store money as integer cents. Empty arrays need `stringList([])`. Details are in [references/typescript.md](references/typescript.md).

## Multi-tenancy

Partitions are fully isolated sub-collections inside one collection: same schema, separate data. Use one partition per tenant, never a `tenant_id` filter, when tenants must not see each other's data.

- Write and query through `client.collection(name, partition)`. Partitions are created on first write.
- A query against one partition never returns another partition's documents.
- Reading a partition that has never been written to raises `PartitionNotFoundError` (TypeScript: `"partition not found"`). Treat it as "no results" for that tenant.
- `delete_partition(name)` (Python) or `deletePartition(name)` (TypeScript) removes a tenant's data. Afterwards, reads of that partition raise "partition not found". **Writes to the same partition name are also rejected for a while**, so don't reuse a deleted tenant's partition name right away.
- **Deletion takes about a minute to apply.** Deleted data stays readable for roughly 60 seconds, and `consistency="strong"` reads can keep returning it longer. Before telling the user a tenant's data is gone, poll with default consistency until the read raises "partition not found" or returns nothing. To list partitions, use `list_partitions()` or `listPartitions()`.

## Recommended workflow

Copy this checklist and track progress:

```
- [ ] 0. If a pattern above matches the task, start from it; otherwise continue
- [ ] 1. Run `python scripts/topk_inspect.py check` to confirm credentials and region
- [ ] 2. Design the schema (one index per field; semantic text ≤ 4,096 chars, so chunk)
- [ ] 3. Create the collection with a fresh name; treat "already exists" as success
- [ ] 4. Upsert in batches; keep the LSN from the last write
- [ ] 5. Query with lsn=<last LSN>, and confirm the count matches what you wrote
- [ ] 6. Print real scores before choosing hybrid weights
```

If a step fails, read the error. Validation errors name the document `_id`, the field, and the reason. Fix the data or schema and rerun.

## Errors

Python raises typed exceptions from `topk_sdk.error`. TypeScript throws a plain `Error`; match on `err.message` as shown.

| Python exception | TypeScript `err.message` | Fix |
|---|---|---|
| `CollectionNotFoundError` | `"collection not found"` | Create it, or check the name and region |
| `CollectionAlreadyExistsError` | `"collection already exists"` | Safe to ignore in setup code |
| `PartitionNotFoundError` | `"partition not found"` | The tenant has no data (never written, or deleted). Return empty results |
| `DocumentValidationError` | contains `DocumentValidationError` | The message names the `_id`, the field, and the reason (`TextTooLong`, `DocumentTooLarge`, `InvalidDataType`, ...). Chunk text, shrink the document, or fix the type |
| `SchemaValidationError` | contains `SchemaValidationError` | The message names the field. Fix the schema |
| `PermissionDeniedError` | `"permission denied"` | Wrong or missing `TOPK_API_KEY`, or the key belongs to another project |
| `RequestTooLargeError` | starts with `"request too large"` | Send fewer documents per upsert |
| `QuotaExceededError`, `SlowDownError` | contains `QuotaExceeded` or `SlowDown` | Back off. The SDK already retries transient errors |
| `QueryLsnTimeoutError` | contains `QueryLsnTimeout` | The write at that LSN wasn't indexed in time. Retry the read |
