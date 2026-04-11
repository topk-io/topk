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
| `--dataset` | Yes      | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                               |
| `--mode`    | No       | Response mode: `auto` (default), `summarize`, `research`                            |
| `--field`   | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`)        |


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
| `--dataset` | Yes      | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                               |
| `--top-k`   | No       | Number of results to return (default: 10)                                           |
| `--field`   | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`)        |


The query can also be piped via stdin:

```bash
echo "my query" | topk search
```

---

### `upload` — Upload files

Accepts a single file path or glob pattern matched against file paths relative to the current directory. Always scans recursively. If exactly one file is matched, you can override the default document ID with `--id` and attach metadata with `--meta`. By default, each uploaded file gets a document ID derived from the SHA-256 of its absolute path.

```bash
topk upload '*.pdf' --dataset my-dataset
topk upload 'docs/**/*.md' --dataset my-dataset
topk upload './report.pdf' --dataset my-dataset --id quarterly-report
```


| Argument    | Required | Description                                                               |
| ----------- | -------- | ------------------------------------------------------------------------- |
| `PATTERN`   | Yes      | A file path or glob pattern matched against relative file paths           |
| `--dataset` | Yes      | Dataset to upload into                                                    |
| `-y`        | No       | Create the dataset automatically if it does not exist & skip confirmation |
| `--id`      | No       | Document ID to assign when exactly one file is uploaded                   |
| `--meta`    | No       | Metadata as a JSON object when exactly one file is uploaded               |
| `-c`        | No       | Number of concurrent uploads, 1–64 (default: 32)                         |
| `--wait`    | No       | Wait for all uploaded files to be fully processed                         |
| `--dry-run` | No       | Preview which files would be uploaded without uploading                   |


In interactive mode, `upload` prompts whether to wait for processing after the files are uploaded. Pass `--wait` to skip the prompt and wait automatically. In non-interactive mode, `upload` returns after upload unless `--wait` is passed.


---

### `list` — List documents in a dataset

```bash
topk list --dataset my-dataset
```

Streams results as they arrive. In agent mode (`-o json`) outputs one JSON object per line (NDJSON).

| Flag        | Required | Description                                               |
| ----------- | -------- | --------------------------------------------------------- |
| `--dataset` | Yes      | Dataset to list documents from                            |
| `--field`   | No       | Metadata field to include (repeatable, e.g. `-f title`)  |

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
