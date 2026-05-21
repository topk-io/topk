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

## Updating the CLI

To update CLI to the latest version, run:

```bash
brew update
brew upgrade topk
```
