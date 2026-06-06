# engram-rs architecture

A local memory layer for the agent driver: capture session activity, serve
on-demand hybrid recall, consolidate into a year-sharded archive, and (deferred)
maintain a global memory file from one LLM call per day.

Local · zero LLM/GPU/network on the read/write hot paths · model downloaded once.

## Documents

- [Capture](capture.md) — hooks, write pipeline
- [Chunking](chunking.md) — heading-bounded, size-capped, fence-safe document chunking
- [Storage](storage.md) — SQLite schema: temporal KG + session archive
- [Recall](recall.md) — hybrid vector + lexical retrieval
- [Consolidation](consolidation.md) — ladder, lazy eviction, archive sharding
- [Memory](memory.md) — the deferred global `MEMORY.md` curation
- [Evaluation](../eval/README.md) — retrieval metrics harness

## Flow

```
IDE ── hooks ──▶ daemon hook event
                      │
                      ▼
               Engram (sole writer) ──▶ Storage (SQLite + FTS5 + embeddings)
                                       ▲           ▲
                                       │           │
IDE ── MCP stdio ──▶ mcp-server ───────┘           │
Browser ── HTTP ──▶ viewer (127.0.0.1:port) ───────┘
```

## Invariants

- Only `Engram` may write to storage; only the storage layer may open the database.
- Hooks do no I/O beyond the `Engram` call.
- Viewer binds to `127.0.0.1` only.
- Embeddings computed out-of-band — never block the write path.
- Documents indexed verbatim.

## Files on disk

```
~/.athing/
  engram.db              active DB (always queried, stays small)
  archive-YYYY.db        year shards (sealed → read-only)
  archive-index.json     shard registry
  models/<name>/         downloaded embedding model (cached)
  memories/MEMORY.md     global memory (deferred curation change)
```

## Crate layout

```
apps/engram-rs/
  Cargo.toml
  docs/                 this directory
  eval/                 retrieval eval harness + golden dataset
    main.rs  corpus.jsonl  queries.jsonl  README.md
  src/
    lib.rs        public Engram API (sole writer)
    store.rs      SQLite open/migrate + all queries
    schema.sql    embedded migration
    embed.rs      Embedder trait, static model, brute-force cosine
    tool_use.rs   PostToolUse skip list + auto-title
    indexer.rs    gitignore-aware scan + markdown chunking
    entity.rs     entity resolution / alias matching
    fact.rs       temporal fact ops (learn / supersede / soft-remove)
    search.rs     vector + lexical + RRF + adaptive weight + recency
    coverage.rs   eviction scoring
    jobs.rs       consolidation ladder
    archive.rs    year-shard router
    mcp.rs        MCP stdio server (recall, expand, entity)
    server.rs     loopback HTTP ingress + viewer
    eval.rs       deterministic IR metrics
    main.rs       CLI
```

### Dependencies

```toml
model2vec-rs = "0.2"                          # static embeddings (model hub, cached)
rusqlite     = { features = ["bundled"] }     # embedded SQLite
ignore       = "0.4"                           # gitignore-aware walk
strsim       = "0.11"                          # fuzzy entity matching
serde / serde_json / anyhow
```

## Public API

```rust
engram.ingest(chunk)                  // raw chunk
engram.capture_prompt(...) / capture_tool(...)
engram.index_project(cwd, ts)         // index project markdown
engram.recall(query, now) -> RecallResult
engram.search(query, now, k) -> Vec<SearchResult>   // one-shot, full content
engram.rank(query, now, k) -> Vec<i64>              // ranked ids (eval)
engram.expand(id) / expand_many(ids)
engram.entity(name) / learn(...) / forget(...)
engram.consolidate(...) / evict(...) / prune_docs() / prune_all()
```

CLI: `status · index · embed · recall · search · expand · archive-recall ·
entity · consolidate · evict · prune · mcp · serve`.

## LLM involvement

Capture, recall, consolidation, and eviction make **no** model call. The only LLM
use is the deferred daily [memory curation](memory.md). Embeddings come from a
static CPU model (downloaded once, then offline).

## Scale expectations

| Metric | Value |
|---|---|
| Active DB vectors | ~22,000 max (policy-bounded) |
| Recall latency | ~1ms (brute-force cosine, measured) |
| Write latency | <150ms per event |
| Active DB size | <50MB (heavy user, indefinitely) |
| Archive growth | ~500MB/year (heavy developer + researcher) |
