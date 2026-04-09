# TopK CLI

Command-line interface for [TopK](https://topk.io) — upload documents, ask questions and search relevant passages grounded in your data.

## Installation

```bash
brew tap topk-io/tap
brew install topk
```

## Configuration

Set `TOPK_API_KEY` and `TOPK_REGION` environment variables before running the CLI:

```bash
export TOPK_API_KEY=<your-api-key>
export TOPK_REGION=<region>
```


| Variable       | Required                | Description                                                                             |
| -------------- | ----------------------- | --------------------------------------------------------------------------------------- |
| `TOPK_API_KEY` | Yes or pass `--api-key` | Your API key. [Get your API key](https://console.topk.io)                               |
| `TOPK_REGION`  | Yes or pass `--region`  | Region where your data is stored. [See available regions](https://docs.topk.io/regions) |


Alternatively, pass `--api-key` and `--region` as flags on any command.

## Commands

### `ask` — Get a grounded answer with citations from documents

```bash
topk ask "my question"
```


| Flag        | Required | Description                                                                         |
| ----------- | -------- | ----------------------------------------------------------------------------------- |
| `--dataset` | No       | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`). Defaults to all datasets.    |
| `--mode`    | No       | Response mode: `auto` (default), `summarize`, `research`                            |
| `--fields`  | No       | Metadata fields to include in results, comma-separated                              |


The query can also be piped via stdin:

```bash
echo "my question" | topk ask
```

---

### `search` — Find relevant passages in documents

```bash
topk search "my query"
```


| Flag        | Required | Description                                                                         |
| ----------- | -------- | ----------------------------------------------------------------------------------- |
| `--dataset` | No       | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`). Defaults to all datasets.    |
| `--top-k`   | No       | Number of results to return (default: 10)                                           |
| `--fields`  | No       | Metadata fields to include in results, comma-separated                              |


The query can also be piped via stdin:

```bash
echo "my query" | topk search
```

---

### `upload` — Upload files matching regex patterns

Accepts one or more regex patterns matched against file paths relative to the current directory. Always scans recursively.

```bash
topk upload '\.pdf$' --dataset my-dataset
topk upload '\.pdf$' '\.md$' --dataset my-dataset
topk upload 'docs/' --dataset my-dataset
```


| Argument    | Required | Description                                                               |
| ----------- | -------- | ------------------------------------------------------------------------- |
| `PATTERNS`  | Yes      | One or more regex patterns matched against relative file paths            |
| `--dataset` | Yes      | Dataset to upload into                                                    |
| `-y`        | No       | Create the dataset automatically if it does not exist & skip confirmation |
| `-c`        | No       | Number of concurrent uploads, 1–64 (default: 32)                         |
| `--wait`    | No       | Wait for all files to be fully processed (agent mode only; default in interactive mode) |
| `--no-wait` | No       | Skip waiting for processing (interactive mode only)                       |
| `--dry-run` | No       | Preview which files would be uploaded without uploading                   |


In interactive mode, upload waits for processing by default — press Enter to skip. In agent mode (`-o json`), pass `--wait` to block until processing completes.

---

### `upsert` — Upsert a single document

```bash
topk upsert --dataset my-dataset --document-id my-doc.pdf ./my-doc.pdf
```


| Argument        | Required | Description                                 |
| --------------- | -------- | ------------------------------------------- |
| `PATH`          | Yes      | Path to the file to upsert                  |
| `--dataset`     | Yes      | Dataset to upsert the file into             |
| `--document-id` | Yes      | Document ID to assign                       |
| `--meta`        | No       | Metadata as `key=value` pairs, repeatable   |
| `--wait`        | No       | Block until the document is fully processed |
| `--dry-run`     | No       | Preview the upsert without uploading        |


---

### `delete` — Delete a document

```bash
topk delete --dataset my-dataset --document-id my-doc.pdf
```


| Flag            | Required | Description                     |
| --------------- | -------- | ------------------------------- |
| `--dataset`     | Yes      | Dataset containing the document |
| `--document-id` | Yes      | Document ID to delete           |
| `-y`            | No       | Skip confirmation prompt        |


---

### `dataset` — Manage datasets

#### List all datasets:

```bash
topk dataset list
```

#### Get a dataset:

```bash
topk dataset get my-dataset
```

#### Create a dataset:

```bash
topk dataset create my-dataset
```

#### Delete a dataset:

```bash
topk dataset delete my-dataset
```

| Argument | Required | Description              |
| -------- | -------- | ------------------------ |
| `DATASET`| Yes      | Dataset name             |
| `-y`     | No       | Skip confirmation prompt |

---

## Output

By default all commands print human-readable text. Pass `-o json` for machine-readable JSON:

```bash
topk -o json dataset list
topk -o json search "query"
```
