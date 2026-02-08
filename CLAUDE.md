# metamodule-experiment

> **Precedence**: This project uses the Cipher-backed context protocol. The "Long-Running
> Agent Protocol" in the global CLAUDE.md is SUPERSEDED by the instructions below.
> Ignore references to `claude-progress.txt`, `feature_list.json`, `session-roundup.md`,
> `/save-progress`, and `/init-harness` — this project uses `/template-resume`, `/template-save`,
> `features.json`, and Cipher memory instead.

> One-sentence purpose: _describe what this project does_

---

## Context Protocol

### Session Start

Run `/template-resume` to load context from Cipher. Fallback: read `.claude/handoff.md` if it exists.
If `session_count` in `.claude/project.json` is 0, this is a fresh project — skip Cipher queries and start with `docs/GOAL.md`.

### During Work

Store critical decisions and failures to Cipher **immediately** when they happen.
Non-critical learnings and minor decisions can wait for `/template-save`.

For immediate stores, use `cipher_extract_and_operate_memory` via `mcp__aggregator__call_tool`:
- Set `memoryMetadata.projectId` to the `name` from `.claude/project.json`
- Set `memoryMetadata.source` to the category name (e.g., `"decision"`, `"failure"`)
- Set `context.sessionId` to `session-<N>` where N is `session_count` from `.claude/project.json`

### Auto-Save

After verified milestones (feature working, tests passing), store a milestone checkpoint
to Cipher **before** committing.

### Session End

Run `/template-save` before ending the session. This stores a handoff + snapshot to Cipher and writes
a local `.claude/handoff.md` backup.

### On-Demand

Query `cipher_memory_search` for past attempts, decisions, rule violations, and learnings
at any point during work. Include the project name in query strings to scope results.

---

## Cipher Categories

| Category | source value | When Stored | Pattern |
|---|---|---|---|
| `handoff` | `handoff` | `/template-save` at session end | ADD (accumulates; resume picks latest by session number) |
| `session-snapshot` | `session-snapshot` | `/template-save` at session end | ADD |
| `milestone` | `milestone` | Feature complete, tests passing | ADD |
| `decision` | `decision` | Architecture/design choice made | ADD |
| `failure` | `failure` | Something broke, approach failed | ADD |
| `experiment` | `experiment` | Tried something, recorded outcome | ADD |
| `rule-violation` | `rule-violation` | Deviated from project rules | ADD |
| `learning` | `learning` | Discovered something non-obvious | ADD |

---

## Cipher Store Pattern

Set `source` to the category name from the table above (e.g., `"decision"`, `"failure"`, `"milestone"`).

```
server: "cipher"
tool: "cipher_extract_and_operate_memory"
input: {
  "interaction": "[category:decision] Chose SQLite over Room because the schema is flat and we don't need migrations. Trade-off: no compile-time query validation.",
  "memoryMetadata": {
    "projectId": "metamodule-experiment",
    "source": "decision"
  },
  "context": {
    "sessionId": "session-3"
  }
}
```

Prefix `interaction` with `[category:NAME]` where NAME is one of the 8 categories above.

---

## Cipher Search Pattern

```
server: "cipher"
tool: "cipher_memory_search"
input: {
  "query": "metamodule-experiment past failures with SSL pinning bypass approach",
  "top_k": 10,
  "include_metadata": true
}
```

Include the project name at the end of query strings to scope results. After retrieval, filter by `projectId` in returned metadata:
1. If `projectId` matches this project name — keep the entry.
2. If `projectId` is missing or doesn't match — discard the entry.
3. If ALL entries are discarded — treat as empty results, fall back to local files.

---

## Phase Gates

Progress through phases in order. Each gate has criteria that must be met before moving on.

| Phase | Gate Criteria |
|---|---|
| **Understanding** | `docs/DOMAIN.md` and `docs/ARCHITECTURE.md` filled. Can explain the system to a newcomer. |
| **Design** | `docs/GOAL.md` and `docs/DESIGN.md` filled. Features listed in `features.json`. |
| **Setup** | Environment working. `docs/SETUP.md` filled with reproduction steps. |
| **Implementation** | One feature at a time from `features.json`. Mark `in_progress` before starting. |
| **Ship** | All MVP features done. Docs complete. README accurate. |

---

## File Map

| Path | Purpose |
|---|---|
| `docs/GOAL.md` | What we're building and why |
| `docs/DOMAIN.md` | The system/platform we're building on |
| `docs/ARCHITECTURE.md` | How the codebase is structured |
| `docs/DESIGN.md` | Technical design and approach |
| `docs/SETUP.md` | Environment setup and build steps |
| `docs/DECISIONS.md` | Decision log (also stored in Cipher) |
| `.claude/features.json` | Feature backlog with status tracking |
| `.claude/project.json` | Project name, session count, current phase |
| `.claude/handoff.md` | Local backup of last session handoff |
| `.claude/commands/extract-context.md` | Collaborative context extraction playbook |
| Cipher `handoff` | Session end state + next steps (per session, latest = highest session number) |
| Cipher `milestone` | Verified progress checkpoints |
| Cipher `decision` | All architecture/design decisions |
| Cipher `failure` | Failed approaches and why |
| Cipher `learning` | Non-obvious discoveries |

---

## Context Extraction Protocol

When analyzing external repos via `/extract-context`:

1. **Use agent teams** — TeamCreate with real-time peer messaging. Not plain subagents.
2. **Mandatory collaboration** — every analyst must message at least one peer during analysis. Isolated work is incomplete work.
3. **Split synthesis** — 2 synthesizers with section ownership. Single-agent synthesis loses detail.
4. **Small task portions** — no agent handles more than 15 files. Split large domains.
5. **Agents persist across phases** — analysts stay alive through synthesis and validation for cross-phase queries.
6. **Active questioning** — synthesizers query analysts for gaps. Validators verify claims with analysts before flagging.
7. **Final consensus** — all analysts sign off on the context file before marking complete.
8. **Minimum 2 per role** — even for small repos. Collaboration IS the quality mechanism.

Full orchestration playbook: `.claude/commands/extract-context.md`

---

## Rules

1. **One feature at a time** -- only one `in_progress` feature in `features.json`
2. **Update features.json on status change** -- set `started_date` / `completed_date`
3. **Store critical items to Cipher immediately** -- blocking decisions and failures, not routine edits
4. **Never overwrite this file** -- this is the static control room
5. **Commit after milestones** -- store Cipher milestone first, then git commit
6. **Read before writing** -- always read existing files before proposing edits
