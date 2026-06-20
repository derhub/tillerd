## Why

The fs backend re-reads and re-parses entity JSON from disk on every `list_*`/get call, and `FsBackend::open` eagerly walks the entire tree to build the id→path index before the orchestrator serves anything. On a large tree both costs grow with the store and are paid repeatedly even when nothing changed. R1b — the next slice after the store relayering (ADR-0035) — removes that waste without changing what callers observe.

## What Changes

- Add an in-memory cache of parsed entity files (workspace/project/session + layout) inside `FsBackend`, keyed by path, validated by file mtime: a read reuses the cached struct when the file's mtime is unchanged and re-reads only when it moved.
- Build the id→path index lazily on first access instead of a full recursive scan during `open`. `seed_defaults` on an empty tree stays eager (it is a write and must run at boot).
- Invalidate/update the affected cache and index entries on the backend's own writes (single-writer model); the per-file/dir mtime compare is the revalidation backstop on read.
- No change to the public store API, return values, or on-disk format — results stay identical, only faster.

## Capabilities

### New Capabilities

- `fs-listing-cache`: the fs backend's mtime-revalidated read-through cache, lazy index construction, and write-driven invalidation — defined as behavior that preserves the existing persistence contract while removing repeated disk reads and the eager boot scan.

### Modified Capabilities

<!-- None — the public persistence contract (workspace-persistence) is unchanged; this slice is internal to the fs backend and behavior-preserving. -->

## Impact

- `crates/orchestrator/src/infra/fs/` — `mod.rs` (`open`, `TreeState`, read/list/index helpers), `index.rs` (lazy build), per-entity read paths, and the write paths that must invalidate. New cache state on `FsBackend`.
- No new crate or dependency; uses `std::fs` metadata mtime.
- Scope excludes the sqlite backend (own query path) and the memory backend (already in-RAM); no external multi-writer coordination beyond the mtime backstop.
