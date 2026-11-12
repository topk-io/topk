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
topk import ./books.parquet [--project-id <PROJECT_ID>] --region aws-us-east-1-elastica
topk import postgres://user:pw@host/db 'public.*' [--project-id <PROJECT_ID>] --region aws-us-east-1-elastica
```

> [!NOTE]
> `topk import` discovers a schema, shows the plan, and imports on confirmation — see [import](#import).

## Authentication

To authenticate, run:

```bash
topk login
```

## Commands

### login

Log in with TopK CLI:

```bash
topk login
```

> [!NOTE]
> If the CLI cannot open your browser automatically, or you prefer to open it manually, use `topk login --no-browser` to print the login URL.

### logout

Remove the saved session:

```bash
topk logout
```

### project

#### list

List projects in the current organization.

```bash
topk project list
```

> [!NOTE]
> `topk project list` marks the currently selected project with `*`.

#### select

Select a default project.

```bash
topk project select  # interactive picker
topk project select <PROJECT_ID>
```

#### clear

Clear the selected project:

```bash
topk project clear
```

#### current

Show the currently selected project.

```bash
topk project current
```

#### get

Get a project by ID.

```bash
topk project get <PROJECT_ID>
```

| argument | description |
| --- | --- |
| `<PROJECT_ID>` | ID of the project |

#### create

Create a project.

```bash
topk project create <NAME>
```

| argument | description |
| --- | --- |
| `<NAME>` | Display name of the new project |

#### delete

Delete a project by ID.

```bash
topk project delete <PROJECT_ID> [-y]
```

| argument | description |
| --- | --- |
| `<PROJECT_ID>` | ID of the project |
| `-y`, `--yes` | Skip the confirmation prompt; required when stdin is not a terminal |

### region

#### list

List available regions.

```bash
topk region list
```

### sql

Create and manage collections, read and write documents, and run keyword, vector, and semantic searches with [TopK SQL](https://docs.topk.io/sdk/topk-sql/overview).

```bash
topk sql <QUERY|--file <FILE>> [OPTIONS]
```

| argument | description |
| --- | --- |
| `[QUERY]` | SQL to run, or `-` to read it from stdin; required unless `--file` is given |
| `-f`, `--file <FILE>` | Read SQL from a file, or `-` for stdin |
| `--region <REGION>` | Region to connect to (env `TOPK_REGION`); required. See https://docs.topk.io/regions |
| `--project-id <PROJECT_ID>` | Project to access with your login (env `TOPK_PROJECT_ID`); defaults to the project selected with `topk project select` |

#### Run a query

Log in and find your project ID, then run SQL against a collection. This example searches a `title` field with a semantic index:

```bash
topk login
topk project list
topk sql "SELECT _id, title, semantic_similarity(title, 'classic American novel') AS score
          FROM books ORDER BY score DESC LIMIT 10" \
  --project-id <project-id> --region aws-us-east-1-elastica
```

See the [TopK SQL reference](https://docs.topk.io/sdk/topk-sql/overview) for complete documentation.

#### Run SQL from a file or stdin

```bash
topk sql -f queries.sql --project-id <project-id> --region aws-us-east-1-elastica
echo "SELECT COUNT(*) FROM books" | topk sql - --project-id <project-id> --region aws-us-east-1-elastica
```


#### Inspect tables

psql-style commands list and describe tables:

```bash
topk sql '\dt' --project-id <project-id> --region aws-us-east-1-elastica          # all tables
topk sql '\dt books*' --project-id <project-id> --region aws-us-east-1-elastica   # matching a pattern
topk sql '\d books' --project-id <project-id> --region aws-us-east-1-elastica     # columns and indexes
topk sql '\di' --project-id <project-id> --region aws-us-east-1-elastica          # all indexes
topk sql '\dA' --project-id <project-id> --region aws-us-east-1-elastica          # index kinds
topk sql '\dT' --project-id <project-id> --region aws-us-east-1-elastica          # column types
topk sql '\?' --project-id <project-id> --region aws-us-east-1-elastica           # supported commands
```

#### Output

Results print as one table per statement by default, with row counts on stderr. Use `-o json` to stream one JSON object per row as it arrives, for example `topk sql … -o json | jq`. Pipe long output to a pager yourself, for example `| less`.

### usage

Show usage of your organization, projects and collections over a time range.

```bash
topk usage --since <TIME> [--until <TIME>] [OPTIONS]
```

| argument | description |
| --- | --- |
| `--since <TIME>` | Start of the range, at most 90 days before the end: a duration ago (`<n>m`, `<n>h`, `<n>d` or `<n>w`), a date (`YYYY-MM-DD`) or an RFC 3339 time; required |
| `--until <TIME>` | End of the range: a duration ago, a date or an RFC 3339 time; defaults to now |
| `--project-id <PROJECT_ID>` | Limit to this project (env `TOPK_PROJECT_ID`); defaults to the whole organization |
| `--collection <NAME>` | Limit to this collection in the project |

#### Choose a time range

Durations count back from now.

```bash
topk usage --since 24h                              # last 24 hours
topk usage --since 7d --until 1d                    # a week, ending a day ago
topk usage --since 2026-10-01 --until 2026-11-01    # October
```

> [!IMPORTANT]
> When a date is passed, it starts at midnight UTC: `2026-10-01` is `2026-10-01T00:00:00Z`.

#### Choose a scope

```bash
topk usage --since 7d --project-id <project-id>                      # a project
topk usage --since 7d --project-id <project-id> --collection books   # a collection
```

> [!IMPORTANT]
> Without `--project-id`, usage covers your whole organization.

#### Metrics

| metric | unit | description |
| --- | --- | --- |
| Writes | count | Write requests |
| Written Data | bytes | Data written |
| Storage | byte-hours | Data stored, over the range |
| Query Requests | count | Query requests |
| Query Memory | byte-hours | Memory used to cache data for queries, over the range |
| Query Compute | bytes | Data scanned by queries |
| Data Egress | bytes | Data returned by queries and GETs |
| GETs | count | GET requests |

### import

Bulk import into TopK collections. Every run prints the plan as a TOML spec and asks before writing; collections are created right after.

```bash
topk import [SOURCE] [OBJECTS]... [OPTIONS]
```

| argument | description |
| --- | --- |
| `[SOURCE]` | Source URL, file path, or glob; also required with `--spec` or `--resume`, except for files |
| `[OBJECTS]...` | Source objects to import; exact names, globs, or `<object>=<collection>` |
| `--region <REGION>` | Region to import into (env `TOPK_REGION`); required. See https://docs.topk.io/regions |
| `--to <COLLECTION>` | Name the target collection; required when it cannot be derived |
| `--id <COLUMN>` | Column to use as the document id (`_id`); use when it can't be auto-detected |
| `--dry-run` | Print the spec and a sample of documents, without importing |
| `-f`, `--spec <SPEC>` | Read a TOML import spec |
| `-y`, `--yes` | Skip the confirmation prompt; required when stdin is not a terminal |
| `--resume <RUN>` | Continue the saved plan by its run ID; cannot be combined with `--spec` |
| `--filter <FILTER>` | Only read rows matching a filter, in the source's language: SQL `WHERE` for files and databases, a JSON query for mongodb/elasticsearch |
| `--limit <LIMIT>` | Read at most this many rows per object |
| `--partition <COLUMN>` | Column whose value names each row's partition; see [Split rows across partitions](#split-rows-across-partitions) |
| `--continue-on-error` | Skip documents that fail instead of stopping; exits non-zero if any did |
| `-c`, `--concurrency <N>` | Concurrent upserts in flight, budgeted across the whole run (default `16`) |
| `--batch-bytes <SIZE>` | Bytes of documents per upsert (default `1MiB`) |
| `--project-id <PROJECT_ID>` | Project to import into with your login (env `TOPK_PROJECT_ID`); defaults to the project selected with `topk project select`. A `--dry-run` needs none |

#### Authenticate

Log in, then provide `--project-id` and `--region` when importing:

```bash
topk login
topk project list
topk import ./books.parquet --project-id <project-id> --region aws-us-east-1-elastica
```

Set `TOPK_PROJECT_ID` and `TOPK_REGION` to omit the corresponding flags. Explicit flags override environment variables.

> [!TIP]
> Use `topk project list` to find your project ID and `topk region list` to list regions. Access tokens are cached and renewed automatically.
> Run `topk project select` once to omit `--project-id` from later commands.

#### Import a database

```bash
topk import postgres://user:pw@host/db 'public.*'
topk import mysql://root@host/shop orders
topk import mongodb://host/shop products
topk import es+https://user:pw@es.example.com 'products*'
topk import postgres://host/db orders --filter "created_at > '2024-01-01'"
```

Objects are exact names, globs, or `<object>=<collection>` renames.

#### Import files

```bash
topk import ./books.parquet                                   # .csv .tsv .json(l) .arrow .avro
topk import 's3://bucket/books/*.parquet' --to books          # a glob needs --to
topk import gs://bucket/books.csv                             # az:// hf:// http(s):// too
```

A single file names its collection.

#### Copy a TopK collection

```bash
topk import topk://<project-id>/books --to books-v2                        # reindex under a new schema
topk import topk://<project-id>/books --region aws-eu-central-1-monstera   # copy across regions
topk import topk://<project-id>/books --project-id <other-project-id>      # copy to another project
```

The source region is resolved from the source collection. `--region` selects the destination region. Schema and indexes copy as-is; the copy is additive. Sources with named partitions are rejected before the destination is created; partitioned collection copies are not supported.

#### Split rows across partitions

```bash
topk import ./events.parquet --partition tenant               # one partition per `tenant` value
```

Each row goes to the partition named by that column, so one source stream writes into as many partitions as the column has values.

> [!TIP]
> The column only has to exist in the source. It does not have to be an imported field.

#### Preview and edit the plan

```bash
topk import postgres://host/db orders --dry-run > spec.toml   # spec on stdout, sample documents on stderr
vim spec.toml                                                 # drop columns, fix types, add indexes
topk import "$DB_URL" -f spec.toml --yes                      # run
```

> [!WARNING]
> Keep credentials on the command line or in the environment, never in the spec — the spec is meant to go in git.

#### Resume a stopped run

Stop a run — `^C`, a lost connection — and pick it up where it left off:

```bash
topk import postgres://host/db --resume 1234abcd               # run id from the resume command
```

Resume skips finished collections and continues unfinished ones from their source checkpoints, after all preceding partition writes succeeded. Limits include rows consumed before the checkpoint, including skipped rows. Without `--resume`, a re-run re-imports everything — upserts are idempotent.

MongoDB checkpoints use the source `_id`, independently of the destination ID mapping. Legacy checkpoints made with a custom ID column require a new import. Elasticsearch retains its point-in-time snapshot for interrupted scans and closes it before returning a final page.

#### Sources

| source | url | filter | auth |
| --- | --- | --- | --- |
| postgres | `postgres://` | SQL `WHERE` | in-URL, `PGPASSWORD` |
| mysql | `mysql://` | SQL `WHERE` | in-URL, `MYSQL_PWD` |
| sqlite | `sqlite:<path>` | SQL `WHERE` | — |
| files | path, glob, `s3://` `r2://` `gs://` `az://` `hf://` `http(s)://` | SQL `WHERE` | credential chain; `hf auth login`; http anonymous |
| elasticsearch | `es://` `es+https://`, `*.cloud.es.io` | query DSL | in-URL, `ELASTIC_API_KEY` / `ELASTIC_PASSWORD` |
| mongodb | `mongodb://` | find document | in-URL, `MONGODB_URI` |
| topk | `topk://<project-id>/<collection>` | — | your login |

SQL column names are literal, including dots. Dots inside schema or table names are not supported; source discovery rejects them explicitly.

#### Spec

One TOML table per collection — `from`, `id`, `filter`/`partition`/`limit` as the flags (`partition` names a column), and a `fields` whitelist (only declared fields import):

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
| `type` | `text` `int` `float` `bool` `bytes` `image` `timestamp` `struct` `*_list` `*_vector` `*_matrix` `*_sparse_vector` |
| `from` | source column, when the name differs |
| `required` | fail the document if missing |
| `truncate` | max chars, text only |
| `dim` / `cols` | vector dimension / matrix row width |
| `index` | `"keyword"` `"exact"` `"semantic"` `"ngram"` `{ vector = { metric = "cosine" \| "euclidean" \| "dot_product" \| "hamming" } }` `{ multi_vector = {} }` |

Discovery takes the source's types; declaring a different `type` converts. Embeddings that arrive as float lists, text (`"[1,2,3]"`) or packed bytes become vectors by declaring `f32_vector` and `dim` (`f32_matrix` and `cols` for multi-vectors); a sparse vector reads numeric keys or parallel `indices`/`values` lists; epoch numbers or date text become `timestamp`. Conversions are exact or the document fails; a narrower declaration (`truncate` included) is how loss is accepted.

Image fields accept JPEG/PNG bytes, data URIs, and Hugging Face `{bytes, path}` structs. Each image must fit within 512 KiB.

Declare an image field with `page = { type = "image", index = "semantic" }` in the spec.

#### Failures and limits

Pending document buffers are bounded per active collection by `--batch-bytes × --concurrency`, independently of in-flight writes and source read-ahead. Under pressure, the importer sends individual partial batches to free space.

Progress counts acknowledged writes. After 10 seconds without progress, the CLI prints a waiting notice.

By default, a row decoding, conversion, or document-validation failure stops the import. `--continue-on-error` skips these rows. If the destination rejects a batch for document validation, the importer retries its documents individually and skips only those rejected. Connection, authentication, query, and stream failures still stop the import after normal retries.

Summaries count acknowledged writes and skipped rows for the current invocation. Any skipped rows or fatal error produce a nonzero exit. Progress counts acknowledged writes for this invocation. Checkpoints advance only after preceding rows are written or skipped. Skips are final for that run: `--resume` does not revisit checkpointed skips. Resuming an interrupted import can replay writes after its last saved checkpoint; upserts replace documents with the same ID. Use a new import to retry corrected rows.

- there is no schema migration: an existing collection whose schema differs is rejected
- an upsert replaces the whole document, so a spec that omits an existing field clears it

## Global flags

These flags are accepted by every command:

### `--output`

Options:

* `text` (default)
* `json`


## Updating the CLI

To update CLI to the latest version, run:

```bash
brew update
brew upgrade topk
```
