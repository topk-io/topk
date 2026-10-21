# `topk` skill evaluations

Three realistic tasks that a coding agent performs against a live TopK project. They were written before the skill, from failures we saw agents hit when they learned TopK from the docs alone, following [Anthropic's evaluation-first guidance](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#build-evaluations-first).

| Eval | What it exercises |
|---|---|
| `docs-hybrid-search-python` | Long text vs. the 4,096-character semantic limit, BM25 + semantic hybrid ranking |
| `product-search-typescript` | `topk-js` schema and typing, filters, optional CLI flags, keyword boost |
| `multi-tenant-rag-python` | Partitions, chunking, tenant isolation, tenant deletion |

## Running

```bash
./setup.sh <eval-id> /tmp/run-1        # builds the fixtures
cp -r ../topk /tmp/run-1/.claude/skills/topk   # omit for a baseline run
cd /tmp/run-1 && claude                 # paste the eval's "query"; set TOPK_API_KEY and TOPK_REGION first
```

Grade the transcript against `expected_behavior` in [`evals.json`](evals.json). Use a unique collection name for every run.

## Results (2026-10-05, `topk-sdk` 0.15.0, `topk-js` 0.15.0)

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

- Subagents can't install skills through the harness, so each run was told the skill's name and description and where its files were, the same information Claude Code shows. Every run chose to load the skill on its own.
- One run per cell. Treat the numbers as directional, not statistically significant.
