# TopK Agent Skills

[Agent Skills](https://agentskills.io) that teach coding agents (Claude Code, Cursor, Codex, GitHub Copilot, and others) to build on TopK correctly.

| Skill | What it covers |
|---|---|
| [`topk`](topk/SKILL.md) | Schema design, loading data, chunking, semantic/keyword/hybrid/vector search, multi-tenant partitions, and errors, with the `topk` CLI (`topk sql`, `topk import`), plus application code with the Python and TypeScript SDKs |

## Install

With the [skills CLI](https://github.com/vercel-labs/skills), which works for most agents:

```bash
npx skills add topk-io/topk --skill topk
```

Or copy the folder manually:

| Agent | Project | Personal |
|---|---|---|
| Claude Code | `.claude/skills/topk/` | `~/.claude/skills/topk/` |
| Cursor | `.cursor/skills/topk/` or `.agents/skills/topk/` | `~/.cursor/skills/topk/` |
| Other agents | `.agents/skills/topk/` | `~/.agents/skills/topk/` |

```bash
git clone --depth 1 https://github.com/topk-io/topk /tmp/topk && cp -r /tmp/topk/skills/topk .claude/skills/
```

The skill uses the [`topk` CLI](https://docs.topk.io/cli) (`brew install topk-io/topk/topk`). The agent runs `topk login` when needed, which opens a browser for the user to log in. The agent loads the skill on its own whenever a task involves TopK.

## Layout

```
topk/
├── SKILL.md              # core rules and workflow (loaded when the skill triggers)
├── references/
│   ├── patterns.md       # proven designs: long docs, per-customer data, catalog search, PDF/Word
│   ├── cli.md            # topk CLI: log in, inspect, run SQL, import and copy data, troubleshooting
│   ├── sql.md            # TopK SQL: tables, writes, search functions, partitions, read-your-writes
│   ├── python.md         # topk-sdk for application code, including an agent retrieval tool
│   ├── typescript.md     # topk-js for application code
│   └── why-topk.md       # how TopK differs from a typical vector database (for "why TopK?" questions)
└── scripts/
    ├── chunk.py          # split text, Markdown, PDF and Word files into chunk JSONL for `topk import`
    └── pdf_pages.py      # render PDF pages to JPEG JSONL for an image semantic index
```

## Development

Validate against the spec:

```bash
pip install skills-ref
agentskills validate skills/topk
```

Every command in `references/` was executed with the `topk` CLI, and the SDK references were checked against the current `topk-sdk` and `topk-js` APIs. The evaluations and their results are in [`evals/`](evals/). When the CLI, TopK SQL or an SDK changes, rerun them and update the skill. Bump `metadata.version` in `SKILL.md` on every change.
