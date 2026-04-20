# TopK CLI

Command-line interface for [TopK](https://topk.io) — upload documents, ask questions and search relevant passages grounded in your data.

## Installation

```bash
brew tap topk-io/tap
brew install topk
```

## Configuration

Run:

```bash
topk login
```

Or set `TOPK_API_KEY` before running the CLI:

```bash
export TOPK_API_KEY=<your-api-key>
```


| Variable       | Required                | Description                                       |
| -------------- | ----------------------- | ------------------------------------------------- |
| `TOPK_API_KEY` | Yes or pass `--api-key` | Your API key. [Get your API key](https://console.topk.io) |


Alternatively, pass `--api-key` as a flag on any command.

To remove the stored API key:

```bash
topk logout
```

## Commands

### `ask` — Get a grounded answer with citations

```bash
topk ask "my question" --dataset my-dataset
```


| Flag        | Required | Description                                                                         |
| ----------- | -------- | ----------------------------------------------------------------------------------- |
| `--dataset` | Yes      | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                               |
| `--mode`    | No       | Response mode: `auto` (default), `summarize`, `research`                            |
| `--field`   | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`)        |
| `--output-dir` | No    | Save result content (images, text chunks) to a directory                            |


The query can also be piped via stdin:

```bash
echo "my question" | topk ask --dataset my-dataset
```

---

### `search` — Find relevant passages in documents

```bash
topk search "my query" --dataset my-dataset
```


| Flag        | Required | Description                                                                         |
| ----------- | -------- | ----------------------------------------------------------------------------------- |
| `--dataset` | Yes      | Dataset to search (repeatable, e.g. `-d ds1 -d ds2`)                               |
| `--top-k`   | No       | Number of results to return (default: 10)                                           |
| `--field`   | No       | Metadata field to include in results (repeatable, e.g. `-f title -f author`)        |
| `--output-dir` | No    | Save result content (images, text chunks) to a directory                            |


The query can also be piped via stdin:

```bash
echo "my query" | topk search --dataset my-dataset
```

---

### `upload` — Upload files

Accepts a single file path, directory, or glob pattern matched against file paths relative to the current directory. Directory inputs scan only that directory by default, and `-r/--recursive` recurses into subdirectories. For glob patterns, `'*.pdf'` matches only the current directory, while `'**/*.pdf'` matches recursively. By default, each uploaded file gets a document ID derived from the SHA-256 of its absolute path.

```bash
topk upload '*.pdf' --dataset my-dataset
topk upload 'docs/**/*.md' --dataset my-dataset
topk upload docs --dataset my-dataset -r
```


| Argument    | Required | Description                                                               |
| ----------- | -------- | ------------------------------------------------------------------------- |
| `PATTERN`   | Yes      | A file path, directory, or glob pattern |
| `--dataset` | Yes      | Dataset to upload into |
| `-r`        | No       | Recurse into subdirectories when `PATTERN` is a directory |
| `-y`        | No       | Skip the upload confirmation prompt |
| `-c`        | No       | Number of concurrent uploads, 1–64 (default: 32) |
| `--wait`    | No       | Wait for all uploaded files to be fully processed |
| `--dry-run` | No       | Preview which files would be uploaded without uploading |
| `--timeout` | No       | Upload timeout in seconds (default: 1800 / 30 minutes) |


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
topk delete --dataset my-dataset --id my-doc-id
```


| Flag        | Required | Description                     |
| ----------- | -------- | ------------------------------- |
| `--dataset` | Yes      | Dataset containing the document |
| `--id`      | Yes      | Document ID to delete           |
| `-y`        | No       | Skip confirmation prompt        |


---

### `dataset` — Manage datasets

#### List all datasets:

```bash
topk dataset list
```

This command has no subcommand-specific flags.

#### Get a dataset:

```bash
topk dataset get my-dataset
```

| Argument  | Required | Description  |
| --------- | -------- | ------------ |
| `DATASET` | Yes      | Dataset name |

#### Create a dataset:

```bash
topk dataset create --region aws-us-east-1-elastica my-dataset
```

| Argument   | Required | Description                     |
| ---------- | -------- | ------------------------------- |
| `DATASET`  | Yes      | Dataset name                    |
| `--region` | Yes      | Region to create the dataset in. List available regions at https://docs.topk.io/regions |

#### Delete a dataset:

```bash
topk dataset delete my-dataset
```

| Argument  | Required | Description              |
| --------- | -------- | ------------------------ |
| `DATASET` | Yes      | Dataset name             |
| `-y`      | No       | Skip confirmation prompt |

---

## Output

By default all commands print human-readable text. Pass `-o json` for machine-readable JSON. The `text` value is also accepted as an alias for the default human-readable format:

```bash
topk -o json dataset list
topk -o json search "query"
```
