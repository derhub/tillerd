## Context

The agent driver captures lifecycle events through a loopback hook ingress. Today those events drive live status only; nothing persists across sessions. We want a local, embedded memory layer that captures sessions and answers on-demand recall — with no API key, GPU, or network on any path, on macOS/Linux for v1.

The full architecture reference lives at `apps/engram-rs/docs/architecture.md` and describes the complete vision including the deferred curation layer. This change implements the capture + recall + consolidation + archive foundation only.

## Goals / Non-Goals

**Goals:**

- One embedded store, one writer, no external services on any path.
- Bounded write latency: capture commits fast; embeddings computed out-of-band.
- Time-aware hybrid recall over conversation history and project docs.
- Indefinitely bounded active-database size via aggregation-only consolidation + lazy eviction to year-sharded archives; nothing permanently deleted.

**Non-Goals:**

- Global memory file, daily curation, temporal fact-graph population, and bootstrap — deferred to the follow-up change `engram-rs-memory-curation` pending the curation-model decision. The fact-graph schema is created here but left empty.
- Code search / structural code indexing (markdown docs only for v1).
- Automatic scheduling of jobs (CLI-triggered here; daemon-scheduled later).
- Replacing or modifying the host agent's own per-project memory files — engram is a parallel, additive layer.
- Multi-user or networked deployment.
- Document change history (the project's version-control system already provides it).

## Decisions

### Embedded SQLite over a vector-database server

The expected scale is small — a policy-bounded active set on the order of tens of thousands of vectors, even for heavy users. An embedded relational store with an in-process ANN index and a full-text index covers semantic + lexical + relational queries in one file, in-process, with no sidecar. A columnar/analytics vector database optimizes batch scans we never run and adds an async, server-shaped dependency we do not want. Trade-off: very large corpora would eventually outgrow this; the consolidation + archive policy keeps the active set bounded so that ceiling is not reached in practice.

### Static, CPU-only embeddings

A static embedding model (vocabulary lookup + pooling) produces vectors in microseconds on CPU with no GPU and no network. This is what makes synchronous-feeling capture and per-session document re-indexing affordable. Trade-off: lower embedding quality than a transformer encoder; acceptable because retrieval is hybrid (lexical recovers what semantics miss) and the corpus is personal-scale.

### Curation deferred; this change is fully model-free

The differentiating layer — a daily model call that rewrites a global memory file and extracts facts — depends on an unresolved question: which model engram calls, given the host's bring-your-own-login premise (no API key). Rather than block the searchable-history foundation on that decision, curation is split into a follow-up change. This change creates the fact-graph schema but leaves it empty and ships zero model dependency. The foundation (capture, recall, consolidation, archive) delivers the core "recover past context" value on its own.

### Out-of-band embeddings keep the write path bounded

Capture commits the chunk and enqueues an embedding request; embedding happens asynchronously. Even though static embedding is fast, decoupling guarantees capture latency never depends on it and a backlog never blocks the agent.

### Aggregation-only consolidation ladder + coverage-driven lazy eviction

Content rolls up a ladder (session → daily → weekly → monthly) by concatenating already-stored content — no model call at any step. When content is rolled into a higher digest it is marked covered. Eviction scores chunks by age, time-since-access, coverage, and access frequency, and moves high scorers to archive in atomic batches. This keeps the active database small and fast while preserving everything — covered, forgotten content leaves the hot set first; uncovered or frequently used content stays. Monthly digests, facts, entities, and relations are never evicted.

### Year-sharded, read-only-sealed archive with on-demand deepening

The archive is partitioned by year; sealed shards open read-only; a registry maps date ranges to shards. Archive search is opt-in (recall first reports uncertainty), searches newest shard first, and deepens on request. This bounds any single archive file and keeps cold queries off the default path.

### Jobs are CLI-triggered now, daemon-scheduled later

Consolidation and eviction are built here but exposed only as on-demand CLI commands (`consolidate`, `evict`). Automatic scheduling needs a long-running supervised process and lands in a later change when engram adopts the daemon. This keeps this change free of a daemon dependency while making the pipeline runnable and testable today.

### Library-free behavioral specs

The capability specs describe behavior generically (embedded store, vector index, full-text index, static embedding model). Concrete library choices live only in this design and the architecture doc, so the contract is not coupled to a dependency.

## Risks / Trade-offs

- **Static embedding quality is lower than a transformer encoder** → Hybrid retrieval (lexical + vector + fusion) recovers matches semantics alone would miss; corpus is personal-scale where the gap is small.
- **Per-session full doc re-index repeats work for unchanged files** → Static embedding is microsecond-scale and indexing is async; the simplicity of delete-and-reinsert outweighs incremental-diff bookkeeping at this scale.
- **Archive growth is unbounded over years** → Year sharding plus size-triggered rotation bounds any single file; cold queries open only the shards a date range needs.
- **Duplicate hook fires could double-store events** → A uniqueness constraint on (session, position, kind) suppresses same-session duplicates while preserving genuinely distinct cross-session content.
- **Empty fact graph until curation lands** → The `entity()` surface returns nothing until the follow-up change populates facts; recall over conversation chunks and docs still works fully.

## Migration Plan

Additive: a new `apps/engram-rs` package and new on-disk state under `~/.athing/`. No existing capability changes, no schema in other packages touched. Rollout is install-and-enable; rollback is disable-and-remove the package and its data directory.

## Open Questions

- Confirm the exact lifecycle-hook names and payload fields exposed by the host agent so capture maps cleanly without path-sniffing.
- (Deferred to the curation change) which model the daily curation job calls given the bring-your-own-login premise, and the size bound / recent-digest window for the global memory file.

## Resolved

- `model2vec-rs` is available on crates.io (0.2.1); `sqlite-vec` is available; an in-process Rust MCP SDK (`rmcp`) is available for the tool server.
