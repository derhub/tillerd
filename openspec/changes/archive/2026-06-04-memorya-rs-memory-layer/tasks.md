## 1. Package scaffold

- [x] 1.1 Create `apps/memorya-rs` crate with `Cargo.toml` and the module layout (`redact`, `embed`, `tool_use`, `indexer`, `entity`, `fact`, `search`, `coverage`, `jobs`, `archive`, `mcp`, `server`, `store`, `lib`); curation modules out of scope; `inject` folded into `lib::context`; `jobs`/`archive` are single files
- [x] 1.2 Add dependencies: bundled embedded SQLite driver, fuzzy-string and JSON helpers (brute-force cosine over BLOB vectors replaces an ANN extension at this scale; the static embedding model is wired behind the `Embedder` trait in a later task)
- [x] 1.3 Build the standalone crate (own `Cargo.toml`/lock, mirroring the existing native daemon crate)

## 2. Storage layer (memorya-storage)

- [x] 2.1 Implement schema migration: `entities`, `facts` (with validity interval + supersession), `relations`, `embeddings` (keyed by item + kind, with model + dim), `sessions`, `chunks`, `digests`, and required indexes
- [x] 2.2 Create the full-text index over chunk and fact content with insert/update/delete-sync triggers
- [x] 2.3 Enforce the single-writer rule: only the storage layer opens the database; expose all writes through one `Engram` writer
- [x] 2.4 Add the uniqueness constraint suppressing same-session duplicate fires while allowing identical cross-session content
- [x] 2.5 Implement temporal fact operations: insert-current, supersede, soft-remove (validity-end), and currently-valid queries
- [x] 2.6 Implement entity resolution + alias matching and `entity(name)` grouping of currently-valid facts

## 3. Embedding (memorya-storage / memorya-capture)

- [~] 3.1 Implement the static CPU embedding wrapper recording model identity and dimensionality — `Embedder` trait + offline `HashingEmbedder` done and records model id + dim; the model2vec-backed embedder is wired behind the trait in a follow-up
- [x] 3.2 Implement the out-of-band embedding worker: drain pending items, embed, write to `embeddings` without blocking the write path
- [x] 3.3 Implement vector search over stored embeddings restricted to non-archived items (brute-force cosine at this scale)

## 4. Capture (memorya-capture)

- [x] 4.1 Implement `<private>` redaction applied before any write
- [x] 4.2 Implement the lifecycle-hook ingress (session start, prompt submit, post-tool, stop, session end) as a loopback HTTP endpoint, fire-and-forget, never blocking the agent
- [x] 4.3 Implement the skip list for low-value tool events
- [x] 4.4 Implement tool-event auto-titling from tool name + primary argument
- [x] 4.5 Implement `ingest`: redact → title → commit chunk → enqueue embedding request
- [x] 4.6 Implement the project document indexer: scan markdown excluding dependency/build/VCS dirs, chunk by heading, replace prior project doc chunks, enqueue embeddings, run async on session start

## 5. Recall (memorya-recall)

- [x] 5.1 Implement hybrid scoring: vector + full-text BM25, fused into one ranked list (RRF)
- [x] 5.2 Implement adaptive weighting (symbol-like → more lexical; prose → more semantic)
- [x] 5.3 Implement temporal rerank (recency boost) and access-stat updates on hit (ended-validity penalty applies to facts, surfaced once the fact graph is populated by the curation change)
- [x] 5.4 Implement `recall(query)` returning `Found({id,title,snippet})` or `Uncertain { offer_archive }` against a confidence threshold
- [x] 5.5 Implement `expand(id)` returning full item content
- [x] 5.6 Implement session-start context assembly (`context`): recent digests with staleness check + project list (global memory comes from the curation change)
- [x] 5.7 Implement the recent-digest staleness check (include only when no newer captured content exists)

## 6. Consolidation & eviction (memorya-consolidation)

- [x] 6.1 Implement `session_end` job: aggregate session chunks into a session-scoped digest, mark chunks covered
- [x] 6.2 Implement daily/weekly/monthly consolidation jobs aggregating the level below, with coverage marking
- [x] 6.3 Implement coverage-debt detection to trigger summarization of sessions with stale uncovered chunks
- [x] 6.4 Implement the eviction score (age, time-since-access, coverage, frequency) and the batched, atomic move-to-archive; never evict monthly digests, facts, entities, relations
- [x] 6.5 Expose consolidation and eviction jobs as CLI commands (manual trigger); automatic daemon scheduling deferred to a later change

## 7. Archive (memorya-consolidation)

- [x] 7.1 Implement the shard registry (`archive-index.json`) and `ArchiveRouter` selecting shards by year, opening sealed shards read-only
- [~] 7.2 Implement year-boundary + size-triggered shard rotation and sealing — year-boundary rotation + sealing done; size-triggered rotation pending
- [x] 7.3 Implement archive search newest-first with progressive deepening, invoked on confirmed archive fallback

## 8. Interfaces

- [x] 8.1 Implement the MCP stdio server exposing `recall`, `expand`, `entity`
- [x] 8.2 Implement the loopback-only HTTP viewer
- [x] 8.3 Define and document the public `Engram` API (`ingest`, `recall`, `expand`, `entity`)
- [x] 8.4 Implement CLI commands to manually run each job: `consolidate` (session/daily/weekly/monthly), `evict`

## 9. Tests

- [x] 9.1 Storage tests: temporal supersession, soft-remove, currently-valid queries, duplicate-fire suppression vs cross-session retention
- [x] 9.2 Capture tests: redaction removes private spans; skip list; out-of-band embedding does not block; doc re-index replaces prior chunks
- [x] 9.3 Recall tests: fusion, adaptive weighting, temporal rerank, staleness check, archive-fallback uncertainty
- [x] 9.4 Consolidation/eviction tests: ladder aggregation, coverage marking, eviction ordering, core-never-evicted, archive move atomicity
