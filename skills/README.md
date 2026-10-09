# TopK Agent Skills

[Agent Skills](https://agentskills.io) that teach coding agents (Claude Code, Cursor, Codex, GitHub Copilot, and others) to build on TopK correctly.

| Skill | What it covers |
|---|---|
| [`topk`](topk/SKILL.md) | Schema design, indexing, chunking, semantic/keyword/hybrid/vector queries, multi-tenant partitions, and errors for the Python SDK, the TypeScript SDK, and SQL |

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

Then set `TOPK_API_KEY` and `TOPK_REGION` in the agent's environment. Create an API key in the [console](https://console.topk.io/api-key), and pick a region from the [regions list](https://docs.topk.io/regions). The agent loads the skill on its own whenever a task involves TopK.

## Layout

```
topk/
├── SKILL.md              # core rules and workflow (loaded when the skill triggers)
├── references/
│   ├── patterns.md       # proven designs: long docs, per-customer data, agent tool, catalog search
│   ├── python.md         # topk-sdk patterns (loaded on demand)
│   ├── typescript.md     # topk-js patterns
│   └── sql.md            # Postgres-wire SQL
└── scripts/
    └── topk_inspect.py   # check credentials, describe a collection (JSON output)
```

## Development

Validate against the spec:

```bash
pip install skills-ref
agentskills validate skills/topk
```

Every code pattern in `references/` was executed against the live API with `topk-sdk` 0.15.0 and `topk-js` 0.15.0. The evaluations and their results are in [`evals/`](evals/). When the SDK changes, rerun them and update the skill. Bump `metadata.version` in `SKILL.md` on every change.
