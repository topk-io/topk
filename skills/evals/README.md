# `topk` skill evaluations

Three realistic tasks that a coding agent performs against a live TopK project. They were written before the skill, from failures we saw agents hit when they learned TopK from the docs alone, following [Anthropic's evaluation-first guidance](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#build-evaluations-first).

| Eval | What it exercises |
|---|---|
| `docs-hybrid-search` | Chunking a file over the semantic limit, import with `-f` field choices, BM25 + semantic hybrid ranking, one result per file |
| `product-search` | Import field types and indexes, filters in `WHERE`, keyword boost |
| `multi-tenant-search` | Partitions from `--partition`, chunking, tenant isolation, partition deletion and polling |
| `cli-import-search` | `topk import -f` instead of a loader, long text vs. the semantic limit (import shortens it silently), `topk sql` searches |

## Running

```bash
./setup.sh <eval-id> /tmp/run-1        # builds the fixtures
cp -r ../topk /tmp/run-1/.claude/skills/topk   # omit for a baseline run
cd /tmp/run-1 && claude                 # paste the eval's "query"; log in with `topk login` first
```

Grade the transcript against `expected_behavior` in [`evals.json`](evals.json). Use a unique collection name for every run.

## Results

### 0.3.0 (2026-11-20, `topk` CLI from `main` at 66e18842)

One headless Claude Code session per cell (`claude -p --model sonnet --setting-sources project`), graded against `expected_behavior`. A failed command is a tool call that printed an error, not counting the expected `Partition does not exist`.

| Eval | Skill | Criteria met | Failed commands | Help lookups | Tool calls | Time | Cost |
|---|---|---|---|---|---|---|---|
| docs-hybrid-search | no | 4/7 (one document per file: the 48k file cut off at 32,768 characters; `CREATE TABLE` instead of `-f` and no `--dry-run`) | 2 (two indexes on one column) | 4 | 10 | 49s | $0.21 |
| docs-hybrid-search | **yes** | **7/7** | 0 | 0 | 6 | 31s | $0.15 |
| product-search | no | 2/5 (a copied `name_kw` column for a BM25 term instead of `boost(match_any)`; `price < 100` compared as an integer) | 4 (`keyword_score`, `@@`, `SELECT DISTINCT`) | 5 | 14 | 59s | $0.23 |
| product-search | **yes** | **5/5** | 0 | 0 | 6 | 21s | $0.14 |
| multi-tenant-search | no | 5/5 (chunked with its own script and loaded with `INSERT`) | 2 (`SUBSTR`, `RETURNING` on a partition delete) | 8 | 17 | 2m25s | $0.28 |
| multi-tenant-search | **yes** | **5/5** | 1 (`SUBSTR`, `SELECT DISTINCT`) | 0 | 11 | 2m12s | $0.19 |
| cli-import-search | no | 4/6 (the 48k article cut off unchecked; no count check) | 0 | 3 | 7 | 24s | $0.11 |
| cli-import-search | **yes** | **6/6** | 0 | 0 | 6 | 31s | $0.14 |

- **Silent truncation decides the outcome.** Both runs without the skill that met a long document let it be cut off at 32,768 characters (one noticed and said so); every run with the skill checked lengths and chunked.
- **Runs without the skill use the CLI too,** from `topk --agent --help` and docs.topk.io, but find the SQL by trial and error and never use `boost` or `match_any`.
- **A deleted partition's search results lag its count.** In both multi-tenant runs, `SELECT COUNT(*)` on the deleted partition failed with `Partition does not exist` while a semantic search on it still returned rows for about 30 seconds. Both runs kept checking until the search failed too.
- The multi-tenant run with the skill still tried `SUBSTR` and `SELECT DISTINCT`: it read `patterns.md` but not `sql.md`, which listed `SUBSTR`. After this run, SKILL.md rule 3 names both.

The results below are for the SDK-based skill (0.2.0), on the earlier SDK versions of the first three evals.

### 0.2.0 (2026-10-05, `topk-sdk` 0.15.0, `topk-js` 0.15.0)

A failed run is a script run that errored or printed wrong results.

| Eval | Model | Skill | Failed runs | Docs pages fetched | Tool calls | Time |
|---|---|---|---|---|---|---|
| docs-hybrid-search-python | Sonnet | no | 2 (`TextTooLong`; stale schema after same-name recreate) | 10 | 25 | 2m40s |
| docs-hybrid-search-python | Sonnet | **yes** | **0** | **0** | 9 | 1m00s |
| product-search-typescript | Sonnet | no | ~9 (CJS top-level await; chained `.index()`; stale schema after recreate; wrong scores from chained `.select()`) | 13 | 37 | 3m38s |
| product-search-typescript | Sonnet | **yes** | **0** | **0** | 9 | 53s |
| product-search-typescript | Opus | **yes** | **0** | **0** | 8 | 1m12s |
| multi-tenant-rag-python | Sonnet | no | 3 (1 error; 2 runs returned a deleted tenant's data) | 10 | 19 | 5m36s |
| multi-tenant-rag-python | Sonnet | **yes** | **0** | **0** | 11 | 1m58s |
| multi-tenant-rag-python | Haiku | **yes** | **0** | **0** | 24 | 3m54s |

With the skill, all five runs met every `expected_behavior` except two:
- **Haiku** waited a fixed 65 seconds for the tenant deletion instead of polling.
- **Sonnet on `docs-hybrid-search-python`** truncated text to fit the semantic field instead of chunking, because the task asked for one document per file. `SKILL.md` now covers this case: index chunks, then keep the best chunk per file.


## Limitations

- 0.3.0 runs are real Claude Code sessions with the skill in `.claude/skills/topk`; every run with the skill loaded it on its own. For 0.2.0, subagents couldn't install skills through the harness, so each run was told the skill's name and description and where its files were, the same information Claude Code shows.
- One run per cell. Treat the numbers as directional, not statistically significant.
