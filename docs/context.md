# Session Context — TopK Docs

## What we worked on

This session was focused on building and refining the Mintlify-based documentation site for TopK, a context engine that turns unstructured documents into reliable context for agents and humans.

---

## Navigation structure (docs.json)

Current tab order:
1. **Documentation** — icon: `/icons/topk-icon.svg`
2. **CLI** — icon: `terminal`
3. **Python SDK** — icon: `/icons/python.svg` (custom SVG)
4. **JavaScript SDK** — icon: `/icons/js.svg` (custom SVG)
5. **Database** — no icon (legacy product, sidelined)

Icon library switched to **Lucide** globally (`"icons": { "library": "lucide" }`). Custom SVGs used for Python and JS since Lucide has no brand icons.

### Documentation tab groups
- **Documentation**: Overview, Quickstart, Regions, Changelog, CLI, TopK MCP
- **Core Concepts**: Document Processing, Ask, Search, Research

### Database tab (legacy)
- **Guides**: semantic-search, vector-search, sparse-vector-search, multi-vector-search, keyword-search, true-hybrid-search, reranking, consistency, multi-tenancy
- **Document API**: upsert, query, get, delete
- **Management API**: create, list, get, delete

---

## Pages created/rewritten

### `introduction.mdx` (Overview)
- Title: "What is TopK?" / Sidebar: "Overview"
- Structure follows Reducto docs pattern:
  1. How it works (Steps: Understand documents, Retrieve evidence, Keep data private)
  2. How to use TopK (cards: Quickstart, CLI, Python SDK, JS SDK, MCP)
  3. Core concepts (cards: Document Processing, Ask, Search, Research)
  4. Use cases (bulleted list with bolds)
  5. Security & Compliance (cards + SOC 2 Type I mention + link to https://trust.topk.io)
  6. Get started (cards)

### `quickstart.mdx`
- Tabs: CLI (terminal icon), Python SDK (custom SVG), JavaScript SDK (custom SVG)
- Steps: install, set API key, upload file (with --region for dataset creation), ask question
- Prerequisites Info block: TopK account + API key

### `mcp.mdx` (TopK MCP)
- Warning: MCP URL is region-specific
- Regions table: elastica → `https://elastica.api.topk.io/mcp`, monstera → `https://monstera.api.topk.io/mcp`
- Setup tabs: Claude Desktop, Claude Code, Cursor, Windsurf, VS Code
- Each uses `npx -y mcp-remote` with `Authorization: Bearer` header via env var
- Available tools: `list_datasets`, `ask`

### `cli.mdx`
- Full CLI reference with right-nav headings for each command
- Commands: `topk ask`, `topk search`, `topk upload`, `topk upsert`, `topk delete`, `topk dataset` (with `list`/`get`/`create`/`delete` as h4 sub-headings)
- Global flags each as `###` headings

### `core/document-processing.mdx`
- Supported formats table (from `ddb-ctx/src/util/mime_type.rs`):
  - `application/pdf`, `text/markdown` (.md, .mdx), `text/html`, `image/png`, `image/jpeg`, `image/gif`, `image/webp`, `image/tiff`, `image/bmp`
- Upload via CLI / Python / JS SDK tabs
- Async processing explained — returns a `handle`
- `check_handle` (one-shot) and `wait_for_handle` (blocking) for Python and JS
- Full end-to-end example

### `core/ask.mdx`, `core/search.mdx`, `core/research.mdx`
- Placeholder pages, to be written

### `regions.mdx`, `changelog.mdx`
- Frontmatter icons added (`globe`, `history`)

---

## Icon decisions

| Location | Icon |
|----------|------|
| Documentation tab | `/icons/topk-icon.svg` (K mark, orange) |
| CLI tab | `terminal` (Lucide) |
| Python SDK tab | `/icons/python.svg` (python-plain outline, currentColor) |
| JavaScript SDK tab | `/icons/js.svg` (Simple Icons JS path, currentColor) |
| Overview page | `/icons/topk-icon.svg` |
| Quickstart | `rocket` |
| Regions | `globe` |
| Changelog | `history` |
| CLI page | `terminal` |
| MCP page | `network` |
| Document Processing | `file-text` |
| Ask | `message-circle` |
| Search | `file-search-2` |
| Research | `flask-conical` |
| Introduction steps | `eye`, `search`, `shield` |

Custom SVG files in `docs/icons/`: `topk-icon.svg`, `topk-k.svg`, `python.svg`, `js.svg`

---

## Key decisions & constraints

- **Lucide icon library** set globally — Font Awesome icons no longer work. Some FA icons have no Lucide equivalent (`python`, `js`, `diagram-project`) → use custom SVGs or nearest Lucide equivalent.
- **Lucide icons confirmed not working**: `circle-help`, `help-circle`, `message-circle-question`, `file-search-corner` — use `message-circle`, `file-search-2` instead.
- **Welcome page removed** from navigation (file still exists but not linked).
- **Database tab** is for the legacy search/vector DB product — sidelined intentionally, no icon.
- **`sunflower` region** is internal only — never use in docs. Only `elastica` (us-east-1) and `monstera` (eu-central-1) for users.
- **MCP host**: `api.topk.io` (not `topk.dev`)
- **Favicon**: `/logo/topk-icon.svg`
- **Existing logo files not modified**: `topk-docs-logo-light.svg`, `topk-docs-logo-dark.svg`
- **SOC 2 Type I** certified — trust center at https://trust.topk.io
- **Processing is async** — `upsert_file` returns a handle; document not immediately queryable.

---

## SDK method names (verified from source)

### Python (`topk_sdk`)
- `client.datasets().create(name)`
- `client.dataset(name).upsert_file(doc_id, path, metadata)` → `resp.handle`
- `client.dataset(name).check_handle(handle)` → `bool`
- `client.dataset(name).wait_for_handle(handle)`
- `client.ask(query, [dataset_names])`

### JavaScript (`topk-js`)
- `client.datasets().create(name)`
- `client.dataset(name).upsertFile(docId, path, metadata)` → `{ handle }`
- `client.dataset(name).checkHandle(handle)` → `boolean`
- `client.dataset(name).waitForHandle(handle)`
- `client.ask(query, [datasetNames])`

### CLI (`topk`)
- `topk upload <pattern> --dataset <name> [-y] [--wait] [--dry-run] [--id <id>] [--meta <json>]`
- `topk ask <query> -d <dataset> [-d <dataset> ...] [--mode auto|summarize|research] [--field f1 --field f2]`
- `topk search <query> -d <dataset> [-d <dataset> ...] [--top-k N] [--field f1 --field f2]`
- `topk delete --dataset <name> --document-id <id> [-y]`
- `topk dataset list|get|create|delete --dataset <name>`
