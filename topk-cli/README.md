# TopK CLI

Command-line interface for [TopK](https://topk.io).

## Installation

```bash
brew tap topk-io/topk
brew install topk
```

## Quick start

```bash
topk login
topk import ./books.parquet --region aws-us-east-1-elastica
topk import postgres://user:pw@host/db 'public.*' --region aws-us-east-1-elastica
```

> [!NOTE]
> `topk import` discovers a schema, shows the plan, and imports on confirmation — see [import](#import).

## Authentication

To authenticate, run:

```bash
topk login
```

Alternatively, you can set `TOPK_API_KEY` environment variable and skip the `topk login` command.

```bash
export TOPK_API_KEY=<your-api-key>
```

## Commands

### login

To authenticate, run:

```bash
topk login
```

Alternatively, you can set `TOPK_API_KEY` environment variable and skip the `topk login` command.

```bash
export TOPK_API_KEY=<your-api-key>
```

### logout

Remove saved credentials:

```bash
topk logout
```

### import

Bulk import into TopK collections. Every run prints the plan as a TOML spec and asks before writing; collections are created right after.

#### Import a database

```bash
topk import postgres://user:pw@host/db 'public.*'
topk import mysql://root@host/shop orders
topk import mongodb://host/shop products
topk import es+https://user:pw@es.example.com 'products*'
```

Objects are exact names, globs, or `<object>=<collection>` renames.

#### Import files

```bash
topk import ./books.parquet                                   # .csv .tsv .json(l) .arrow .avro .xlsx
topk import 's3://bucket/books/*.parquet' --to books          # a glob needs --to
topk import gs://bucket/books.csv                             # az:// hf:// http(s):// too
```

A single file names its collection. S3 resolves your AWS profile via the aws CLI. `xlsx` reads the first sheet.

#### Copy a TopK collection

```bash
topk import topk://aws-us-east-1-elastica/books --to books-v2                        # reindex under a new schema
topk import topk://aws-us-east-1-elastica/books --region aws-eu-central-1-monstera   # copy us → eu
```

Schema and indexes copy as-is. `--filter` is not supported; the copy is additive.

#### Preview and edit the plan

```bash
topk import postgres://host/db orders --dry-run > spec.toml   # spec on stdout, sample documents on stderr
vim spec.toml                                                 # drop columns, fix types, add indexes
topk import "$DB_URL" -f spec.toml --yes                      # run
```

> [!WARNING]
> Keep credentials on the command line or in the environment, never in the spec — the spec is meant to go in git.

#### Resume a stopped run

```bash
topk import postgres://host/db --resume 01J9...               # run id printed at start
```

Resume skips finished collections and continues the in-flight one from a checkpoint. Without `--resume`, a re-run re-imports everything — upserts are idempotent.

#### Filter, limit, partition

```bash
topk import postgres://host/db orders --filter "created_at > '2024-01-01'"
topk import ./books.parquet --limit 1000 --partition acme
```

`--filter` applies to one object — use `filter` per collection in a spec for several; `--limit` and `--partition` apply to every object.

#### Sources

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

#### Spec

One TOML table per collection — `from`, `id`, `filter`/`partition`/`limit` as the flags, and a `fields` whitelist (only declared fields import):

```toml
[books]
from = "public.books"
id = "sku"

[books.fields]
title = { type = "text", index = "semantic" }
year = { type = "int", from = "published_year" }
embedding = { type = "f32_vector", dim = 768, index = { vector = { metric = "cosine" } } }
```

| field key | |
| --- | --- |
| `type` | `text` `int` `float` `bool` `bytes` `timestamp` `struct` `*_list` `*_vector` `*_matrix` `*_sparse_vector` |
| `from` | source column, when the name differs |
| `required` | fail the document if missing |
| `truncate` | max chars, text only |
| `dim` / `cols` | vector dimension / matrix row width |
| `index` | `"keyword"` `"exact"` `"semantic"` `"ngram"` `{ vector = { metric = "cosine" \| "euclidean" \| "dot_product" \| "hamming" } }` `{ multi_vector = {} }` |

#### Declaring types

Discovery takes the source's types. Declare a different one to convert:

| source column | discovers as | declare |
| --- | --- | --- |
| float array (sql `FLOAT[]`, json list) | `float_list` | `f32_vector`, `dim` |
| pgvector `vector(n)` | `text` | `f32_vector`, `dim` |
| packed embedding blob | `bytes` | `f32_vector` etc., `dim` |
| flat list of `rows * cols` floats (colbert) | `float_list` | `f32_matrix`, `cols` |
| epoch millis/seconds, RFC 3339 or `YYYY-MM-DD` text | `int` / `text` | `timestamp` |

Conversions are exact or the document fails; a narrower declaration (`truncate` included) is how loss is accepted.

#### Failures and limits

A document fails when a value does not convert exactly, its id is missing or empty, or it is over 200 KB. A failure stops the run (`--resume` continues); `--continue-on-error` skips instead, reports the ids, and exits non-zero.

- there is no schema migration: an existing collection whose schema differs is rejected
- an upsert replaces the whole document, so a spec that omits an existing field clears it

## Global flags

These flags are accepted by every command:

### `--output`

Options:

* `text` (default)
* `json`

Output results as NDJSON — one JSON object per line, compatible with `jq`.

### `--api-key`

API key to use for this invocation. Overrides the `TOPK_API_KEY` environment variable and the key saved via `topk login`.

### `--region`

Region to connect to (env `TOPK_REGION`); required by `import`. See https://docs.topk.io/regions.

## Updating the CLI

To update CLI to the latest version, run:

```bash
brew update
brew upgrade topk
```
