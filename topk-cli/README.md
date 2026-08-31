# TopK CLI

Command-line interface for [TopK](https://topk.io).

## Install

```bash
brew tap topk-io/topk
brew install topk
```

## Quick start

Log in once, then point `topk import` at a file or database:

```bash
topk login
topk import ./books.parquet --region aws-us-east-1-elastica
topk import postgres://user:pw@host/db 'public.*' --region aws-us-east-1-elastica
```

> [!NOTE]
> `topk import` discovers a schema, shows the plan, and imports on confirmation — see [topk import](#topk-import).

## Authentication

```bash
topk login     # saves an API key for later runs
topk logout    # removes it
```

`TOPK_API_KEY` or `--api-key` take precedence over the saved key.

## topk import

Bulk import into TopK collections. Every run discovers the source, prints the plan as a TOML spec, and asks for confirmation; collections are created right after.

```
topk import [OPTIONS] [SOURCE] [OBJECTS]...
```

Sources ([details](#sources)):

- databases — Postgres, MySQL, SQLite
- MongoDB and Elasticsearch
- files — csv, json(l), parquet, arrow, avro, xlsx — local, S3, GCS, Azure, Hugging Face, http(s)
- other TopK collections

`-y` skips the confirmation; `--id <column>` sets the id column when detection fails.

### Import a database

```bash
topk import postgres://user:pw@host/db 'public.*'
topk import mysql://root@host/shop orders
topk import sqlite:~/books.db
topk import mongodb://host/shop products
topk import es+https://user:pw@es.example.com 'products*'
topk import postgres://host/db public.users=people            # rename
```

Objects are exact names, globs, or `<object>=<collection>` renames.

### Import files

```bash
topk import ./books.parquet                                   # .csv .tsv .json .jsonl .ndjson .arrow .avro .xlsx
topk import './data/*.parquet' --to parts                     # a glob needs --to
topk import 's3://bucket/books/*.parquet' --to books          # r2:// too
topk import gs://bucket/books.csv                             # or gcs://
topk import az://container/books.jsonl                        # or azure://
topk import 'hf://datasets/stanfordnlp/imdb/plain_text/train-*.parquet' --to imdb
topk import https://example.com/books.csv
```

A single file names its collection. S3 also resolves your AWS profile via the aws CLI (SSO and role chaining included). `xlsx` reads the first sheet.

### Copy a TopK collection

```bash
topk import topk://aws-us-east-1-elastica/books --to books-v2                        # reindex under a new schema
topk import topk://aws-us-east-1-elastica/books --region aws-eu-central-1-monstera   # copy us → eu
```

Schema and indexes copy as-is; vectors arrive bit for bit. `--filter` is not supported; the copy is additive.

### Preview

Prints the spec on stdout and sample documents on stderr; nothing is written:

```bash
topk import postgres://host/db orders --dry-run
```

> [!TIP]
> No login or region is needed to preview an import.

### Save the plan and re-run it

```bash
topk import postgres://host/db orders --dry-run > spec.toml   # capture
vim spec.toml                                                 # drop columns, fix types, add indexes
topk import "$DB_URL" -f spec.toml --yes                      # run
```

The spec holds what to import; the command line holds where to connect.

> [!WARNING]
> Keep credentials on the command line or in the environment, never in the spec — the spec is meant to go in git.

### Resume a stopped run

```bash
topk import postgres://host/db orders
# run 01J9...
# ^C
topk import postgres://host/db --resume 01J9...
topk import postgres://host/db --resume 01J9... -f spec.toml   # with an edited spec
```

Resume skips finished collections and continues the in-flight one from where it stopped:

- files continue from the last row offset
- databases from the last imported id (ordered by the id column)
- Elasticsearch from a server-side cursor kept for 24 hours — once expired, that collection restarts
- topk from the last `_id`

A collection with a `limit` is never checkpointed, so it restarts. Without `--resume`, a re-run re-imports everything — upserts are idempotent.

> [!NOTE]
> With `-f`, cursors are kept only for collections whose spec block is unchanged; resume assumes the source did not change.

### Filter, limit, partition

Narrow a run without touching the source:

```bash
topk import postgres://host/db orders --filter "created_at > '2024-01-01'"
topk import mongodb://host/shop products --filter '{"active": true}'
topk import ./books.parquet --limit 1000
topk import ./books.parquet --partition acme
```

- `--filter` — which rows import, in the source's query language ([per source](#sources)). Applies to one object; use `filter` per collection in a spec for several.
- `--limit` — caps rows per collection; applies to every object, spec included
- `--partition` — imports into the given partition; applies to every object, spec included

### Output

```bash
topk import ./books.parquet --yes -o json | jq '.books.rows'
```

`-o json` prints `{"<collection>": {"rows": N, "failed": N}}`.

> [!NOTE]
> Rows sharing an id collapse into one document, so `rows` can exceed the document count.

### Sources

Each source's URL scheme, filter language and credentials:

| source | url | filter | auth |
| --- | --- | --- | --- |
| postgres | `postgres://` | SQL `WHERE` | in-URL, `PGPASSWORD` |
| mysql | `mysql://` | SQL `WHERE` | in-URL, `MYSQL_PWD` |
| sqlite | `sqlite:<path>` | SQL `WHERE` | — |
| files | path, glob, `s3://` `r2://` `gs://` `az://` `hf://` `http(s)://` | SQL `WHERE` | credential chain; `hf auth login`; http anonymous |
| elasticsearch | `es://` `es+https://`, `*.cloud.es.io` | query DSL | in-URL, `ELASTIC_API_KEY` / `ELASTIC_PASSWORD` |
| mongodb | `mongodb://` | find document | in-URL, `MONGODB_URI` |
| topk | `topk://[<key>@]<region>/<collection>` | — | URL key, else the run's key |

Any http(s) URL that is not Elasticsearch is treated as a file, named by its extension; query strings are stripped, so presigned URLs work.

> [!NOTE]
> Elasticsearch and MongoDB discover vector types automatically (from mappings / by sampling documents); every other source follows [Declaring types](#declaring-types).

### Spec

The plan every run prints — one TOML table per collection. Save it with `--dry-run`, edit, and run it with `-f`:

```toml
[books]
from = "public.books"
id = "sku"
filter = "published"
partition = "acme"
limit = 10000

[books.fields]
title = { type = "text", index = "semantic" }
isbn = { type = "text", index = "keyword", required = true }
year = { type = "int", from = "published_year" }
summary = { type = "text", truncate = 2000 }
embedding = { type = "f32_vector", dim = 768, index = { vector = { metric = "cosine" } } }
colbert = { type = "f32_matrix", cols = 128, index = { multi_vector = {} } }
```

| collection key | |
| --- | --- |
| `from` | source object (table, index, collection, file) |
| `id` | column that becomes `_id`; several fields may read the same column |
| `filter`, `partition`, `limit` | as the flags |
| `fields` | whitelist — only declared fields import; a spec with no fields is rejected |

| field key | |
| --- | --- |
| `type` | see below |
| `from` | source column, when the name differs |
| `required` | fail the document if missing |
| `truncate` | max chars, text only |
| `dim` | vector dimension; required to vector-index, decodes packed bytes |
| `cols` | matrix row width; a flat list becomes a matrix |
| `index` | `"keyword"` `"exact"` `"semantic"` `"ngram"` `{ vector = { metric = "cosine" \| "euclidean" \| "dot_product" \| "hamming" } }` `{ multi_vector = { quantization = "1bit" \| "2bit" \| "scalar" } }` |

Types: `text` `int` `float` `bool` `bytes` `timestamp` `struct` `text_list` `int_list` `float_list` `{f32,f16,f8,u8,i8,binary}_vector` `{f32,f16,f8,u8,i8}_matrix` `{f32,f16,f8,u8,i8}_sparse_vector`.

### Declaring types

Discovery takes the source's types. Declare a different one to convert:

| source column | discovers as | declare |
| --- | --- | --- |
| float array without a known dim (sql `FLOAT[]`, json list) | `float_list` | `f32_vector`, `dim` |
| pgvector `vector(n)` | `text` (`"[1,2,3]"`) | `f32_vector`, `dim` |
| packed embedding blob (little-endian) | `bytes` | `f32_vector` etc., `dim` |
| flat list of `rows * cols` floats (colbert) | `float_list` | `f32_matrix`, `cols` |
| epoch millis/seconds, RFC 3339 or `YYYY-MM-DD` text | `int` / `text` | `timestamp` |
| decimal wider than f64 | `text` (exact) | `float` to accept loss |

For packed blobs the element width is `len / dim`, so one declaration reads f16/f32/f64 blobs; `u8`/`i8`/`binary` read one byte per element.

Conversions are exact or the document fails: `"3.00"` → `int` 3, `"3.50"` → error; a narrower declaration (`truncate` included) is how loss is accepted. A wrong `dim` that divides the blob evenly decodes silently wrong — check the byte length in `--dry-run`.

### Failures and limits

A document fails when:

- a value does not convert exactly to its declared type
- its id is missing, null, empty or non-finite
- it is over 200 KB

A failure stops the run (`--resume` continues); `--continue-on-error` skips instead, reports the ids, and exits non-zero. Connection, auth and name-collision errors always stop. Throttling retries for up to an hour — lower the upsert concurrency (`-c`) if you hit that.

Limits:

- collection names match `^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$`; two objects mapping to one name is an error, rename with `=`
- field names cannot start with `_`
- a composite primary key produces a placeholder `id` that refuses to run until edited
- an id-only table matched by a glob is skipped; nothing importable left is an error
- no schema migration: an existing collection whose schema differs is rejected — use a new name, or delete it first
- an upsert replaces the whole document, so a spec that omits an existing field clears it; the run names those fields before the prompt

## Global options

| flag | env | |
| --- | --- | --- |
| `--api-key <KEY>` | `TOPK_API_KEY` | overrides the key saved by `topk login` |
| `--region <REGION>` | `TOPK_REGION` | required by `import`; see https://docs.topk.io/regions |
| `--host <HOST>` | `TOPK_HOST` | endpoint is `<REGION>.api.<HOST>`; default `topk.io` |
| `--https [true\|false]` | `TOPK_HTTPS` | default `true` |
| `-o, --output text\|json` | | `json` puts results on stdout for `jq` |
| `-v, --verbose` | `RUST_LOG` | log to stderr |
| `--agent` | `CLAUDECODE` `AGENT` | `--help` includes this manual |

## Upgrade

```bash
brew upgrade topk
```
