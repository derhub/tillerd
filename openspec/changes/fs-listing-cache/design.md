## Context

`FsBackend` (`crates/orchestrator/src/infra/fs/`) backs the durable product store on disk. Today `FsBackend::open` runs `seed_defaults` on an empty tree and then `build_index` — a full recursive walk of `workspaces/` (live + `.archive/`) producing an id→path `HashMap` held in `TreeState`. Every `list_*`/get call then reads and re-parses the relevant JSON files from disk; no parsed struct is cached. Both costs scale with the tree and recur even when nothing changed.

The store is single-writer: only this orchestrator process mutates the tree (workspace-persistence — "read and written only by the Rust backend"). The on-disk format and the public store API are part of the frozen 0.0.6 data model; this slice is internal and behavior-preserving.

## Goals / Non-Goals

**Goals:**

- Avoid re-parsing unchanged entity files on repeated reads.
- Avoid the eager full-tree scan at `open`; build the index on demand.
- Keep read results and on-disk format byte-identical to today.

**Non-Goals:**

- Caching in the sqlite backend (own query path) or memory backend (already in-RAM).
- Multi-writer / external-process coordination beyond the mtime backstop.
- Any change to the public store API, return types, or disk layout.

## Decisions

- **Cache keyed by path, validated by mtime.** Store parsed entity files (`WorkspaceFile`/`ProjectFile`/`SessionFile`/`LayoutFile`) in a `HashMap<PathBuf, (SystemTime, Parsed)>` on `FsBackend`. A read `stat`s the file; equal mtime → return the cached struct; differing/absent → re-read, re-parse, replace the entry. The single `stat` per read is far cheaper than parse, and the mtime compare is the correctness backstop against out-of-band edits.
  - *Alternative — invalidate only on own writes, no mtime check:* rejected; a stale cache after any external touch would return wrong data with no recovery. mtime keeps correctness cheap.
  - *Alternative — content hash instead of mtime:* rejected; hashing re-reads the file, defeating the purpose. mtime is the standard cheap revalidation signal.
- **Lazy index, eager seed.** `open` keeps `seed_defaults` (a write that must exist at boot) but defers `build_index`. The id→path index is populated on first access — resolving an id walks from the root as needed and records what it learns, so cold get-by-id works without a prior full scan. Listings populate the index for the dirs they touch.
- **Write-through invalidation.** Each mutating method already holds the write lock; it updates or removes the affected cache entry and index mapping in the same critical section, so a read after a write never depends on the mtime backstop for its own changes.

## Risks / Trade-offs

- **mtime granularity / clock skew** → on filesystems with coarse mtime, two writes within the same tick could be missed by an external observer. Mitigated: own writes are write-through (not mtime-dependent); the mtime path only covers out-of-band edits, which are out of the single-writer model anyway.
- **Lazy index correctness for archived entities** → get-by-id must still resolve entities under `.archive/` (needed for hard-delete). Mitigated: the on-demand resolver searches live and archived subtrees exactly as `build_index` did, just scoped to the lookup.
- **Cache memory growth** → unbounded path cache on a huge tree. Acceptable for the current scale (one user's workspaces); eviction is a later concern, not this slice.

## Open Questions

None — scope and invalidation model resolved before proposing.
