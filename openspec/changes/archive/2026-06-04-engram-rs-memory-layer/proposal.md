## Why

Coding-agent CLIs forget everything between sessions: conversation history is lost once a session ends, and there is no way to ask "what did we do about X last week?" A local memory layer that captures sessions and surfaces relevant history on demand lets any driven agent recover past context without an API key, GPU, or network call.

## What Changes

- Add a new `apps/engram-rs` package: a local, embedded memory layer backing the agent driver.
- Capture every session event (prompts, tool executions) via lifecycle hooks into an embedded SQLite store, out-of-band embedding so the write path never blocks.
- Provide on-demand semantic recall (vector + lexical hybrid) over conversation history and project docs, exposed as MCP tools with progressive disclosure.
- Consolidate raw session content up a ladder (session → daily → weekly → monthly digests) by aggregation only — no model call — and lazily evict cold content to year-sharded archive databases; nothing is ever permanently deleted.
- Index project markdown so the agent can answer questions about a project regardless of what it has read this session.
- This change has zero LLM, GPU, or network dependency on any path. The temporal fact-graph schema is created but left empty; its population, the daily global-memory curation, and bootstrap are **deferred to a follow-up change** (`engram-rs-memory-curation`) pending the curation-model decision.
- **BREAKING**: none — this is an additive new package with no changes to existing capabilities.

## Capabilities

### New Capabilities

- `engram-storage`: embedded SQLite schema and storage invariants — temporal fact graph (warm-tier schema, populated by the follow-up change), session chunks and digests (cold tier), out-of-band embeddings table, full-text indexes, single-writer rule.
- `engram-capture`: lifecycle-hook ingestion and the write pipeline — redaction, chunking, auto-titling, duplicate-fire suppression, and per-session project-doc indexing.
- `engram-recall`: hybrid retrieval (vector ANN + lexical, fused, temporally reranked) exposed as MCP tools with progressive disclosure and archive fallback.
- `engram-consolidation`: the aggregation-only consolidation ladder, coverage tracking, lazy eviction scoring, and year-sharded archive with on-demand deepening.

### Modified Capabilities

<!-- none — additive new package -->

## Impact

- New package `apps/engram-rs` (Rust): depends on a static CPU embedding library, a bundled embedded SQLite driver, an in-process vector-search extension, and fuzzy-string and JSON helpers.
- New on-disk state under `~/.athing/`: `engram.db`, year-sharded `archive-YYYY.db`, and `archive-index.json`. The global memory file is introduced by the follow-up curation change.
- New MCP server surface (`recall`, `expand`, `entity`) over stdio, plus a loopback-only HTTP viewer.
- Consumes the existing lifecycle-hook ingress; adds no changes to the engine or adapter contracts.
- Zero LLM, GPU, or network on any path in this change.
