# Capture

How session activity and project docs enter the store.

## Hooks

| Hook | Timing | Action |
|---|---|---|
| `SessionStart` | session open | inject context (MEMORY.md + recent digests + project list); trigger doc indexer async |
| `UserPromptSubmit` | user sends prompt | capture prompt as a `chunk` |
| `PostToolUse` | after every tool | capture as a `tool` chunk; skip noise tools |
| `Stop` | agent finishes | trigger session-end consolidation |
| `SessionEnd` | session closes | cleanup |

All hooks are fire-and-forget HTTP to the daemon and never block the agent.

**Skip list** (low-value tools, not stored):
```
ListMcpResourcesTool  SlashCommand  Skill  TodoWrite  AskUserQuestion
```

**Tool auto-title** (for `kind='tool'` chunks):
```
Read  src/auth.rs       →  "Read src/auth.rs"
Edit  src/auth.rs       →  "Edit src/auth.rs"
Bash  cargo test --lib  →  "Bash cargo test --lib"
```

## Write pipeline

```
UserPromptSubmit:
  → ingest: INSERT chunk + FTS5 trigger        (sync, <150ms)
  → emit embedding_pending                     (async, out-of-band)

PostToolUse (non-skipped):
  → skip-list check
  → auto-title
  → ingest tool chunk
  → emit embedding_pending

Stop:
  → session_end: aggregate uncovered chunks → INSERT digest scope='session'
```

Documents are indexed verbatim. See [chunking](chunking.md).

The host agent's own memory-file writes are captured as ordinary tool chunks;
engram does not intercept or special-case them.

## Out-of-band embedding

Capture commits the chunk synchronously and enqueues an embedding request; a
worker embeds later. Write latency never depends on embedding computation.
