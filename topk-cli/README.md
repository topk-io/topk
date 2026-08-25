# TopK CLI

Command-line interface for [TopK](https://topk.io) — upload documents, ask questions and search relevant passages grounded in your data.

## Installation

```bash
brew tap topk-io/topk
brew install topk
```

## Authentication

To authenticate, run:

```bash
topk login
```

Alternatively, you can set `TOPK_API_KEY` environment variable and skip the `topk login` command.

```bash
export TOPK_API_KEY=<your-api-key>
```

<!-- manual:start -->

## Commands

### ask

Get a grounded answer with citations

```bash
topk ask "my question" --dataset my-dataset
```

| Flag           | Required | Description                                                                  |
| -------------- | -------- | ---------------------------------------------------------------------------- |
| `--dataset`    | **Yes**  | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                         |
| `--mode`       | No       | Response mode: `auto` (default), `summarize`, `research`                     |
| `--field`      | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`) |
| `--show-refs`  | No       | Show citations inline in the answer                                          |
| `--output-dir` | No       | Save result content (images, text chunks) to a directory                     |


The query can also be piped via stdin:

```bash
echo "my question" | topk ask --dataset my-dataset
```

---

### search

Find relevant passages in documents

```bash
topk search "my query" --dataset my-dataset
```

| Flag           | Required | Description                                                                  |
| -------------- | -------- | ---------------------------------------------------------------------------- |
| `--dataset`    | **Yes**  | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                         |
| `--top-k`      | No       | Number of results to return (default: 10)                                    |
| `--field`      | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`) |
| `--output-dir` | No       | Save result content (images, text chunks) to a directory                     |


The query can also be piped via stdin:

```bash
echo "my query" | topk search --dataset my-dataset
```

### upload

Upload files to a dataset

```bash
topk upload '*.pdf' --dataset my-dataset
topk upload 'docs/**/*.md' --dataset my-dataset
topk upload docs --dataset my-dataset -r
topk upload report.pdf notes.md images/ --dataset my-dataset
```


| Argument    | Required | Description                                                              |
| ----------- | -------- | ------------------------------------------------------------------------ |
| `PATTERN`   | **Yes**  | One or more file paths, directories, or glob patterns                    |
| `--dataset` | **Yes**  | Dataset to upload into                                                   |
| `-r`        | No       | Recurse into subdirectories when `PATTERN` is a directory                |
| `-y`        | No       | Skip the upload confirmation prompt                                      |
| `-c`        | No       | Number of concurrent uploads, 1–64 (default: 32)                         |
| `--wait`    | No       | Wait for processing; optionally up to a duration (e.g. `--wait 5m`)     |
| `--dry-run` | No       | Preview which files would be uploaded without uploading                  |


### import

Import tables, indexes, collections and files into TopK collections — from Postgres, MySQL, SQLite, MongoDB, Elasticsearch, and csv/json(l)/parquet/arrow/avro/xlsx files (local, S3, GCS, Azure, hugging face, or http(s)).

```bash
topk import postgres://user:pw@host/db 'public.*'        # discover, confirm, import
topk import ./books.parquet                               # files name themselves
topk import './data/*.parquet' --to parts                 # a glob cannot
topk import postgres://host/db public.users=people        # rename inline
topk import postgres://host/db --dry-run > spec.toml      # spec to edit
topk import postgres://host/db -f spec.toml --yes         # run it
```

| Argument / flag       | Required | Description                                                                     |
| --------------------- | -------- | ------------------------------------------------------------------------------- |
| `SOURCE`              | **Yes*** | Source URL, file path, or glob (*also required with `--spec`, except for files) |
| `OBJECTS`             | No       | Objects to import: exact names, globs, or `<object>=<collection>` renames       |
| `-f, --spec`          | No       | Run a TOML import spec                                                          |
| `--dry-run`           | No       | Print the spec (stdout) and sample docs (stderr), import nothing                |
| `--to`                | No       | Name the target collection (single object only)                                 |
| `--partition`         | No       | Import into this partition                                                      |
| `--filter`            | No       | Only read rows matching a filter (single object only)                           |
| `--limit`             | No       | Read at most this many rows *per object*; combines with `--spec`                |
| `-y, --yes`           | No       | Skip confirmation                                                               |
| `--continue-on-error` | No       | Skip documents that fail; exit non-zero if any did                              |
| `--resume`            | No       | Continue a run that stopped, by the id in its header; `-f` swaps in an edited spec |

#### The spec is the plan

A run discovers, prints the spec, and confirms; what you approve is what you can save, edit, and re-run. The spec and prompt are UI (stderr); `--dry-run` prints the spec on stdout and sample docs on stderr — so `--dry-run > spec.toml` captures the spec alone, with no login, no region, and nothing written.

```toml
[books]
from = "public.books"
id = "sku"

[books.fields]
title = { type = "text" }  # index = "keyword" | "exact" | "semantic"
embedding = { type = "f32_vector", dim = 768 }  # index = { vector = { metric = "cosine" } }
```

- The confirmation always states the searchability outcome (`# indexed: title ("semantic")`, or a warning when no indexes are declared). A wrong index is the most expensive available mistake — there is no schema migration; the fix is delete and reimport.
- **The spec is a whitelist.** Only declared fields import; deleting a line excludes the column. A spec with no `[c.fields]` is rejected — it would import ids alone. Several fields may read one column, the id included.
- **Credentials never enter a spec.** The spec says *what* to import; the command line says *where to connect*. Source-native env still works (PGPASSWORD / MYSQL_PWD / MONGODB_URI / ELASTIC_API_KEY \| ELASTIC_PASSWORD). The scheduled shape is `topk import "$DB_URL" -f spec.toml --yes` — secret in the environment, spec in git.
- **Multi-vectors are flat.** A source that writes one flat `FLOAT[]` per document (colbert's shape) becomes a matrix by declaring `cols`; rows follow from the length. `f32_matrix` + `cols` + `index = { multi_vector = {} }`.
- **Vectors are declared by the source, or by you.** A vector index needs `f32_vector` + `dim`. Sources that declare dims discover them; the rest emit `float_list` with a `# declare f32_vector + dim to vector-index` hint — you know your dim. A binary column works the same way: declare the vector type and `dim`, and the packed bytes decode.
- **Conversions are exact or they error.** Decimals wider than f64 discover as `text` and keep their exact value; u64 stays u64; strings with an all-zero fraction parse exactly (`"3.00"` declared `int` → 3). Declaring the wider or narrower type is how loss is accepted: `float` over a wide decimal, f16 over f64, `truncate = <chars>` on a text field.
- **Rows, not docs.** Upsert collapses rows sharing an id (last write wins), so the summary's row count can exceed the collection's document count. `-o json` carries `rows`, `failed`, `bytes`, timings.
- **Runs resume.** Every run prints `# run <id>` before it starts and checkpoints as upserts land; `topk import <source> --resume <id>` continues it — finished collections skipped, the in-flight one from its cursor. The plan is stored (`config_dir/topk/import/<id>.toml`, deleted on success), so only the source and credentials go on the command line. Cursors are the source's own: files `<file>:<row offset>` (rows read in stored order, one duckdb thread), databases the last id (rows ordered by the id column — postgres sorts on its side via `postgres_query`, mysql and sqlite sort in duckdb), Elasticsearch its PIT + `search_after` (24h; expired → that collection restarts). `--resume <id> -f spec.toml` runs an edited spec, keeping cursors only for collections whose block is unchanged. Resume assumes the source did not change: rows added to a finished collection are not picked up. Without `--resume`, a re-run re-imports everything — upserts are idempotent, so that is always correct, just not cheap.

#### Sources

| source | filter language | vectors | auth |
| --- | --- | --- | --- |
| postgres, mysql, sqlite | SQL `WHERE` | `float_list` + hint, or packed binary | in-URL, PGPASSWORD / MYSQL_PWD |
| files: csv, json(l), parquet, avro, xlsx; local, S3, GCS, Azure, hugging face, http(s) | SQL `WHERE` | `float_list` + hint, or packed binary | object-store credential chain; S3 also resolves your AWS profile via the aws CLI (SSO + role chaining, refreshed while the run lives); `hf auth login` for hugging face; http is anonymous |
| elasticsearch | query DSL | mapping declares `dims` (`/8` for bit) | in-URL, ELASTIC_API_KEY \| ELASTIC_PASSWORD |
| mongodb | find document | `$sample: 100` lengths all agree → `f32_vector` | in-URL, MONGODB_URI |

```bash
topk import postgres://user:pw@host/db 'public.*'
topk import mysql://root@host/shop orders
topk import sqlite:~/books.db
topk import mongodb://host/shop products
topk import es+https://user:pw@es.example.com 'products*'
topk import ./books.parquet                                    # or .csv .tsv .json .jsonl .ndjson .arrow .avro .xlsx
topk import 's3://bucket/books/*.parquet' --to books           # r2:// too
topk import gs://bucket/books.csv                              # or gcs://
topk import az://container/books.jsonl                         # or azure://
topk import 'hf://datasets/stanfordnlp/imdb/plain_text/train-*.parquet' --to imdb
topk import 'hf://datasets/org/name@~parquet/default/train/*.parquet' --to name
topk import https://example.com/books.csv
```

Elasticsearch is addressed explicitly: `es://host` / `es+https://host` (long form `elasticsearch://`), and Elastic Cloud domains (`*.cloud.es.io`, `*.elastic.cloud`) are recognized from bare https urls. Any other http(s) url is a file, named by its extension (query strings are stripped, so presigned urls work).

Hugging face is addressed by file, not by dataset name; `@~parquet` after the name reads hugging face's converted parquet copy of a repo whose own files we cannot read.

pgvector columns discover as `text` — the vector type and dim only exist in Postgres's catalog. Declare `f32_vector` + `dim` in the spec and the `"[1,2,3]"` strings import as vectors.

Embeddings are often stored as one packed binary blob per row rather than a list of numbers. Such a column discovers as `bytes`; declare a vector type and `dim` and it decodes:

```toml
embedding = { type = "f32_vector", dim = 2560, index = { vector = { metric = "cosine" } } }
```

The bytes are read little-endian, and the element width comes from the data — `len / dim`, which must divide exactly. So the same declaration reads an f16 blob (5120 bytes), an f32 blob (10240) or an f64 blob (20480), converting to the type you declared; `u8_vector`, `i8_vector` and `binary_vector` read one byte per element and refuse anything wider. Because `dim` decides how the bytes are cut, a wrong `dim` that happens to divide evenly decodes silently wrong — `--dry-run` prints each binary cell's length (`<5120 bytes: 5d 8f ec …>`), which is the number to divide.

#### Failures

Every failure either skips its unit or stops the run, decided by scope:

| scope | failures | behavior |
| --- | --- | --- |
| document | coerce, id, oversize, required-missing, per-row codec — all detected client-side | default: abort, `--resume` continues; `--continue-on-error`: skip, report ids, exit non-zero |
| object | id-only table matched by a glob | skip with a note; error only if nothing importable remains |
| run | connection, auth, upsert batch; composite key, name collision | stop everything — a partial import that exits 0 is the worst outcome available; `--resume` continues |
| throttle | shard capacity exceeded (`SlowDown`) | never fails: the batch backs off (250ms → 8s) in its inflight slot, shedding load; a warning every minute says so — lower `-c`, or Ctrl-C and `--resume` |

Read errors name the object and `--filter`; connect errors redact the DSN password; a missing spec file names its path.

#### Limits

- **Names**: collections match `^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$`; two objects mapping to one name is an error suggesting the inline `=` rename. Field names are non-empty with no leading `_`.
- **Ids**: a composite primary key produces a placeholder spec that refuses to run until edited — `--dry-run` says so too. Null, empty, or non-finite ids fail that document.
- **Size**: documents over 200 KB fail client-side, naming the id and the largest fields; the error points at `truncate` for text, and at indexing an embedding instead for binary — half an image is not a smaller image.
- **Schema drift** is checked before the prompt: a mismatch says to use a new collection name or delete the collection and re-run.
- **Nothing read, nothing created**: collections are created at the first flush, so a typo'd filter or `--limit 0` leaves nothing behind.
- Deliberately out of scope: rebuilding a collection (delete it, or import under a new name), and xlsx beyond the first sheet.


### list

List documents in a dataset:

```bash
topk list --dataset my-dataset
```

Streams results as they arrive. In agent mode (`-o json`) outputs one JSON object per line (NDJSON).


| Flag        | Required | Description                                             |
| ----------- | -------- | ------------------------------------------------------- |
| `--dataset` | **Yes**  | Dataset to list documents from                          |
| `--field`   | No       | Metadata field to include (repeatable, e.g. `-f title`) |


### delete

Delete a document from a dataset:

```bash
topk delete --dataset my-dataset --id my-doc-id
```


| Flag        | Required | Description                     |
| ----------- | -------- | ------------------------------- |
| `--dataset` | **Yes**  | Dataset containing the document |
| `--id`      | **Yes**  | Document ID to delete           |
| `-y`        | No       | Skip confirmation prompt        |


### dataset

Manage datasets

#### list

List all datasets:

```bash
topk dataset list
```

This command has no subcommand-specific flags.

#### get

Get a dataset:

```bash
topk dataset get my-dataset
```


| Argument  | Required | Description  |
| --------- | -------- | ------------ |
| `DATASET` | **Yes**  | Dataset name |


#### create

Create a dataset:

```bash
topk dataset create --region aws-us-east-1-elastica my-dataset
topk dataset create --region aws-us-east-1-elastica --description "My dataset" my-dataset
```


| Argument        | Required | Description                                                                                                             |
| --------------- | -------- | ----------------------------------------------------------------------------------------------------------------------- |
| `DATASET`       | **Yes**  | Dataset name                                                                                                            |
| `--region`      | **Yes**  | Region to create the dataset in. List available regions at [https://docs.topk.io/regions](https://docs.topk.io/regions) |
| `--description` | No       | Dataset description                                                                                 |

#### update

Update a dataset:

```bash
topk dataset update my-dataset --description "My dataset description"
```


| Flag            | Required | Description                                  |
| --------------- | -------- | -------------------------------------------- |
| `DATASET`       | **Yes**  | Dataset name                                 |
| `--description` | No       | Dataset description                          |


#### delete

Delete a dataset:

```bash
topk dataset delete my-dataset
```


| Argument  | Required | Description              |
| --------- | -------- | ------------------------ |
| `DATASET` | **Yes**  | Dataset name             |
| `-y`      | No       | Skip confirmation prompt |


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

Log out and clear cache:

```bash
topk logout
```

## Global flags

These flags are accepted by every command:

### `--output`

Options:

* `text` (default)
* `json`

Output results as NDJSON — one JSON object per line, compatible with `jq`:

```bash
topk -o json dataset list | jq '.name'
```

### `--api-key`

API key to use for this invocation. Overrides the `TOPK_API_KEY` environment variable and the key saved via `topk login`.

<!-- manual:end -->

## Updating the CLI

To update CLI to the latest version, run:

```bash
brew update
brew upgrade topk
```
