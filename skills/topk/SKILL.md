---
name: topk
description: Builds search and retrieval on TopK, a hosted search database with built-in semantic search, BM25 keyword search, vector search, filtering, and multi-tenant partitions, using the `topk` CLI, TopK SQL and the Python and TypeScript SDKs. Use when creating TopK collections, loading data into TopK (from files, Postgres, MySQL, MongoDB, Elasticsearch, SQLite, S3, GCS or Hugging Face), running semantic, keyword, hybrid, vector, or per-tenant searches with `topk sql`, inspecting or copying collections, writing application code with the `topk-sdk` Python or `topk-js` TypeScript package, or when the user asks how TopK compares to other vector databases.
license: MIT
metadata:
  author: topk-io
  version: "0.3.0"
---

# TopK

TopK stores documents in **collections** (SQL tables) and searches them with SQL: score in `SELECT`, filter in `WHERE`, rank with `ORDER BY … LIMIT`. Text fields with a `semantic_index()` are embedded and reranked on the server, so no embedding provider is needed.

Use the `topk` CLI for everything except application code: `topk sql` creates collections, writes, and searches; `topk import` loads existing data. Don't write a loader or a one-off script for TopK. Use the SDKs only in the user's application, for example a search endpoint or an agent's retrieval tool.

## Start from a proven pattern

If the task matches one of these, start from the pattern in [references/patterns.md](references/patterns.md) and adapt it. Each one avoids mistakes that raise no error: unsearchable text, leaked tenant data, stale chunks after re-indexing, scanned pages that are never found.

| Task | Pattern |
|---|---|
| Search docs, articles or other long text (chunking, semantic + BM25, one result per file) | 1. Search long documents |
| Keep each customer's data separate, including deleting a customer | 2. Per-customer partitions |
| Product or catalog search with filters, price ranges and keyword boosts | 3. Catalog search |
| Index PDF or Word files, with page or section citations, including scanned pages | 4. PDF and Word files |

## References

- **CLI** (install, login, inspect, import, copy, troubleshooting): [references/cli.md](references/cli.md)
- **SQL** (tables, writes, search functions, partitions, read-your-writes): [references/sql.md](references/sql.md)
- **Application code:** Python (`topk-sdk`, including an agent retrieval tool) in [references/python.md](references/python.md), TypeScript (`topk-js`) in [references/typescript.md](references/typescript.md)

If something isn't covered here, read the docs as markdown: index at https://docs.topk.io/llms.txt, and any page by appending `.md` to its URL.

If the user asks what TopK is, how it differs from a vector database or search engine, or whether it fits their use case, read [references/why-topk.md](references/why-topk.md) and answer from it.

## Setup

```bash
topk --version || brew install topk-io/topk/topk
topk project list -o json     # "not logged in": run `topk login` first (see below)
```

- **Log in once with `topk login`.** It opens a browser and waits up to 5 minutes, so tell the user first and run it in the background or with a longer timeout ([cli.md](references/cli.md#log-in-and-select-a-project) covers machines without a browser).
- **Keep the selected project and region.** If none is selected, select one as in [cli.md](references/cli.md#log-in-and-select-a-project), always passing the value: without one, `topk project select` and `topk region select` open an interactive picker.
- **Never run a command that waits for input.** Pass `--yes` to `topk import` and `topk project delete`.
- **`-o json` prints one JSON object per line** (NDJSON), not an array.
- `TOPK_PROJECT_ID` and `TOPK_REGION` in the environment override the selected project and region. If they point somewhere unexpected, run `env -u TOPK_PROJECT_ID -u TOPK_REGION topk …`, or pass `--project-id` and `--region`.

Look at what's there before designing anything:

```bash
topk sql '\dt'                 # collections
topk sql '\d <collection>'     # fields and indexes
topk sql "SHOW PARTITIONS FROM TABLE <collection>"
```

## Look at the data first

Before choosing a schema, see what the data is:

```bash
topk import ./articles.jsonl --to articles --dry-run      # field names, discovered types, sample rows
jq '.body | length' articles.jsonl | sort -n | tail -1    # longest text (SQL: SELECT max(length(body)) …)
```

From that, decide which fields to search (semantic or keyword), which to filter on, which to keep only for display, and whether long text needs chunking. Ask the user only what the data can't tell you, such as which fields people will search or how one tenant is identified. If nobody can answer (a CI job, a batch run), choose, and say what you chose and why in your summary.

## Design the schema

TopK is schemaless by default. Declare only the fields you index or want type-checked; other fields can be stored on any document.

Choose **one** index per field:

| Need | Index | Notes |
|---|---|---|
| Natural-language search over text | `semantic_index()` | Server-side embeddings and reranking. **Also supports BM25**, so one field covers hybrid search. Max **32,768 characters** per value. `topk import` can also put it on image fields (JPEG/PNG, up to 512KB). |
| Keyword-only (BM25) search over text | `keyword_index()` | No length cap beyond the document size. |
| Your own embeddings | `vector_index(metric = 'cosine')` on `f32_vector(dim)` and similar | Metrics: `cosine`, `euclidean`, `dot_product`, `hamming` (binary). |
| Late-interaction (ColBERT-style) | `multi_vector_index(metric = 'maxsim')` on `f16_matrix(dim)` and similar | |
| Substring or regex filters | `ngram_index()` | Speeds up `contains` and `~`. |

```sql
CREATE TABLE IF NOT EXISTS docs (
  source TEXT NOT NULL INDEX keyword_index(),
  text   TEXT NOT NULL INDEX semantic_index()
);
```

The schema is fixed once the collection exists; there's no `CREATE INDEX` or `ALTER TABLE`. To change it, copy the data into a new collection with new field choices: `topk import topk://docs --to docs_v2 -f source=keyword,text=semantic` (see [references/cli.md](references/cli.md#copy-a-collection)). Name collections with letters, digits and `_`; a name with `-` must be double-quoted in every statement.

## Rules that cause most failures

1. **Chunk long text.** A value in a `semantic_index()` field can be at most 32,768 characters. `INSERT` fails with `TextTooLong { … max_length: 32768 … }`, but **`topk import` silently cuts the value off at 32,768 characters**, so the rest of the document is never found. Its preview prints `warning: <field> over 32768 characters will be shortened` for every semantic text field, whether or not any value is that long, so check the longest value yourself (see "Look at the data first"). One embedding of a whole long document also matches questions about any one part of it poorly. Split long documents into passages of about 2,000–3,500 characters, one document per chunk, and import the chunks; don't truncate to fit. [Pattern 1](references/patterns.md#1-search-long-documents) shows chunking and re-indexing changed files.
2. **Give every searched field an index with `-f`.** `topk import` stores every field, but indexes only those named with `-f`; without it, nothing is searchable. `-f source=keyword,text=semantic` sets the indexes, and `--exclude raw_html` leaves a field out. Always run the same command with `--dry-run` first, and read the plan and its warnings.
3. **Every search ends with `ORDER BY <score> … LIMIT k`.** `ORDER BY` without `LIMIT` is rejected. Give scoring functions an alias and sort by it: `semantic_similarity(text, 'q') AS score … ORDER BY score DESC`. `SELECT *` isn't supported; list the fields. Neither are `SELECT DISTINCT` (use `GROUP BY` with an aggregate) or string functions such as `SUBSTR` and `lower`: select the whole field and shorten it with `-o json | jq '.text |= .[0:200]'`.
4. **`bm25_score()` requires `match(…)` or `should(…)` in `WHERE`,** on a field with a keyword or semantic index. `match(q, field)` keeps only documents that contain a query term. When keyword terms should *boost* without excluding documents, use `should(q, field)`.
5. **Scores aren't normalized.** `semantic_similarity` and `bm25_score` have different, data-dependent ranges. Before choosing hybrid weights (`ORDER BY sem + kw * 0.2 DESC`), look at real scores with `-o json`. Semantic search always returns its nearest neighbours, even when nothing is relevant; if "no results" is the right answer for an off-topic query, calibrate a minimum score from real queries.
6. **Sort direction depends on the score.** Semantic similarity, BM25, and cosine or dot-product vector scores are higher-is-better: `DESC`. Euclidean distance is lower-is-better: `ASC`.
7. **Don't drop and recreate a collection under the same name.** For about 60 seconds after `DROP TABLE`, writes to a same-name recreate can still go to the dropped collection and are then lost. Use a new name (`docs_v2`), delete documents with `DELETE … WHERE` instead, or wait 60 seconds. Use `CREATE TABLE IF NOT EXISTS` so re-running a setup is safe.
8. **Read your own writes with the LSN, not `sleep`.** `INSERT … RETURNING _lsn` returns the write's LSN; read with `SELECT … FROM docs WITH (required_lsn = <LSN>) …`. Without it, a search right after a write can miss it.
9. **Every document needs a string `_id`.** `INSERT` with an existing `_id` replaces the whole document. To change some fields and keep the rest, use `UPDATE … WHERE _id = …`.
10. **Literals must match the declared type.** An integer literal into a float field fails (`InvalidDataType … expected_type: "float", got_value: "i64"`): write `99.0`, or store money as integer cents. Compare timestamps with `TIMESTAMP '2024-01-01'`; a plain string matches nothing.
11. **Ask before deleting or replacing data you didn't create.** Before `DROP TABLE`, a `DELETE` that matches more than a few known `_id`s, `DELETE FROM docs$<tenant>`, `topk project delete`, or an import into an existing collection (it replaces documents with the same `_id`, and `--exclude` clears that field on them), tell the user what will be deleted or replaced and wait for a yes. Collections you created in this session for testing can be dropped without asking.
12. **Size limits:** one document is at most 2MB including generated embeddings, one write request at most 8MB, and one semantic query string at most 4,096 characters. `topk import` batches for you; with `INSERT`, send a few hundred rows per statement.

## Multi-tenancy

Partitions are fully isolated sub-collections inside one collection: same schema, separate data. Use one partition per tenant, never a `tenant_id` filter, when tenants must not see each other's data.

- Address a partition as `docs$acme` (or `docs PARTITION acme`) in `SELECT`, `INSERT`, `UPDATE` and `DELETE`. Partitions are created on first write; with `topk import`, `--partition <column>` writes each row to the partition named by that column.
- A query against one partition never returns another partition's documents.
- Reading a partition that has never been written fails with `Partition does not exist`. Treat it as "no results" for that tenant.
- `DELETE FROM docs$acme` (no `WHERE`) drops the whole partition. **It takes about a minute to apply,** and the data stays readable meanwhile, so check until it's gone before telling the user ([pattern 2](references/patterns.md#2-keep-each-customers-data-separate)). Don't reuse a deleted tenant's partition name right away.
- `SHOW PARTITIONS FROM TABLE docs` lists partitions.

## Plan limits

Each organization has quotas: the number of projects, collections per project, documents per collection, and requests per second per collection. Free organizations have low limits, paid ones higher, and some have custom limits. The errors name the limit:

| Error | Meaning |
|---|---|
| `Collection limit reached: the project can have at most N collections` | Reuse or drop one of your own test collections, or ask the user |
| `Project limit reached: the organization can have at most N projects` | Use an existing project |
| `quota exceeded: Collection size limit exceeded` | The collection holds its maximum number of documents |
| `Too many requests` | Too many requests per second. For `topk import`, rerun the printed `--resume` command with a lower `-c`; for queries, slow down |
| `You have run out of credits` / `Organization access is blocked` | Only the user can fix this, in the console |

Don't work around a quota by splitting data across more collections or deleting the user's data. Stop, tell the user which limit was hit, and point them to https://console.topk.io/quotas to request more.

## Recommended workflow

Copy this checklist and track progress:

```
- [ ] 0. If a pattern above matches the task, start from it; otherwise continue
- [ ] 1. `topk project list -o json` and `topk sql '\dt'` to confirm login, project and region
- [ ] 2. Look at the data (`--dry-run` without `-f`, longest text), then design the schema (one index per field; chunk long text)
- [ ] 3. Load existing data with `topk import <source> --to <collection> -f <field>=<index>,… --dry-run`, check the plan, then rerun it with `--yes` instead of `--dry-run`;
         or a few documents with `CREATE TABLE IF NOT EXISTS` + `INSERT … RETURNING _lsn`
- [ ] 4. Confirm `SELECT COUNT(*)` matches what you loaded
- [ ] 5. Search with `topk sql -o json`; print real scores before choosing hybrid weights
```

If a command fails, read the error: `topk` prints `error: …` and exits non-zero. Validation errors name the document `_id`, the field, and the reason (`TextTooLong`, `InvalidDataType`, …). Fix the data or schema and rerun. More errors and fixes are in [references/cli.md](references/cli.md#troubleshooting).
