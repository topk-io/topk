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


| Flag        | Required | Description                                                                          |
| ----------- | -------- | ------------------------------------------------------------------------------------ |
| `--sources` | No       | Datasets to search, comma-separated. If omitted, searches across all datasets        |
| `--mode`    | No       | Response mode: `auto` (default), `summarize`, `reason`, `deep-research`              |
| `--fields`  | No       | Metadata fields to include in results, comma-separated                               |


The query can also be piped via stdin:

```bash
echo "my question" | topk ask
```

---

### `search` — Find relevant passages in documents

```bash
topk search "my query"
```


| Flag        | Required | Description                                                                          |
| ----------- | -------- | ------------------------------------------------------------------------------------ |
| `--sources` | No       | Datasets to search, comma-separated. If omitted, searches across all datasets        |
| `--top-k`   | No       | Number of results to return (default: 10)                                            |
| `--fields`  | No       | Metadata fields to include in results, comma-separated                               |


---

### `upload` — Upload files

Accepts a comma-separated list of files or directories. Supported formats:

- Documents: `pdf`, `md`, `mdx`, `html`, `htm`
- Images: `png`, `jpeg`, `jpg`, `gif`, `webp`, `tiff`, `tif`, `bmp`

```bash
topk upload ./docs/ --dataset my-dataset
topk upload ./doc1.pdf,./doc2.pdf --dataset my-dataset
```


| Argument    | Required | Description                                                               |
| ----------- | -------- | ------------------------------------------------------------------------- |
| `PATHS`     | Yes      | Comma-separated list of files or directories to upload                    |
| `--dataset` | Yes      | Dataset to upload into                                                    |
| `-y`        | No       | Create the dataset automatically if it does not exist & skip confirmation |
| `-r`        | No       | Scan directories recursively                                              |
| `-c`        | No       | Number of concurrent uploads, 1–64 (default: 32)                         |
| `--wait`    | No       | Block until all uploaded files are fully processed                        |
| `--dry-run` | No       | Preview which files would be uploaded without uploading                   |


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
topk dataset get --dataset my-dataset
```

| Argument    | Required | Description      |
| ----------- | -------- | ---------------- |
| `--dataset` | Yes      | Dataset name     |

#### Create a dataset:

```bash
topk dataset create --dataset my-dataset
```

| Argument    | Required | Description          |
| ----------- | -------- | -------------------- |
| `--dataset` | Yes      | Name of the dataset to create |

#### Delete a dataset:

```bash
topk dataset delete --dataset my-dataset
```

| Argument    | Required | Description      |
| ----------- | -------- | ---------------- |
| `--dataset` | Yes      | Dataset to delete |
| `-y`        | No       | Skip confirmation prompt |

---

## Output

By default all commands print human-readable output. Pass `--json` anywhere before the subcommand for machine-readable JSON:

```bash
topk --json dataset list
topk --json search "query"
topk --json --pretty search "query"
```
