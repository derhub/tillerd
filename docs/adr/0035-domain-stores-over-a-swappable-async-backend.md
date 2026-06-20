# 0035. Domain stores over a swappable async backend (enum dispatch); hand-editable files + listing cache

- Status: proposed
- Date: 2026-06-19

## Context

ADR-0033 made the domain plane a readable JSON snapshot tree as the source of truth, with a
3-way `merge3` reconcile for git-style versioning and per-entity baselines. One fact breaks that
model:

- **Scale.** Even on the desktop, sessions reach the thousands. Listing or filtering a file tree
  is O(N) -- every list reads and parses every file, and boot re-scans the whole tree. There are
  no indexes. The slice-2 id->path index fixes id-lookup and boot but not listing/filtering.
  Indexes are the fix, and indexes mean a database.

The readability motivation also narrows: **hand-editable** is wanted; **git-versioned/mergeable**
is not. Dropping git removes the reason for `merge3`/reconcile/baselines -- a single occasional
editor, no concurrent merges.

This supersedes ADR-0033.

## Decision

Access domain entities through **per-entity async store structs over a swappable, enum-dispatched
backend**, and replace file-scan listing with an indexed cache.

- **Per-entity stores over an enum backend -- no trait objects.** A closed `enum Backend { Fs,
  Sqlite, Memory }` wraps the concrete backends; each entity has a concrete
  async store struct (`Workspaces`, `Projects`, `Sessions`, `Surfaces`) that holds a `Backend` and
  dispatches by `match`. Async fns work natively on the concrete structs -- no `async-trait`, no
  object-safety, no generic-associated-type gymnastics. Operations take a declarative, typed
  per-entity `Filter` the backend pushes down (SQL `WHERE` / scoped walk) -- never a closure (no
  pushdown). The closed backend set fits enum dispatch (rust-best-practices for a known set); a
  trait would be introduced only for open/plugin backends.
- **Swappable, assigned at the composition root.** Which `Backend` serves which entity is wired per
  host: domain -> `fs` (hand-editable files), operational -> `sqlite`, tests -> `memory`.
  `DomainStore`/`OperationalStore`/`CompositeStore` are dissolved. Sync backends
  wrap via `spawn_blocking`.
- **Files are hand-editable, not a versioning artifact.** Drop `merge3`, 3-way reconcile, and
  per-entity baselines. Hand-edits are detected by `mtime`: a boot-time O(N) `stat` pass (stats
  are far cheaper than open+parse; thousands cost milliseconds) refreshes changed entries; open
  re-stats the one file; the currently-visible set is re-stat'd on view focus/refresh so the
  active list stays fresh. Zero watchers; off-screen hand-edits reconcile at the next boot or
  refresh (bounded staleness).
- **Fast listing via a KV cache, not `state.db`.** The `fs` adapter serves listing and filtering
  from a key-value cache -- an in-memory KV of entity metadata (`id, parent, title, sortOrder,
  status, mtime`) on the hot path, persisted (a KV store, or at most one dedicated `state.db`
  table) so boot loads the cache instead of re-reading the tree. List/filter run in memory over
  the cached metadata; full entity content lazy-loads on open. The cache is derived and
  disposable -- rebuilt from the files and validated by `mtime`. `state.db` (ADR-0034) keeps its
  operational runtime-state role and is **not** the cache.
- **fs-vs-DB-as-truth is a backend detail behind the per-entity stores.** Desktop may keep
  files-as-truth + cache, or move to sqlite-as-truth. The store API is
  stable across all of these, so the choice is deferrable and reversible.

## Consequences

- Listing and filtering are indexed and scale to thousands of sessions; boot does not read the
  tree (a `stat` pass plus the cache).
- The merge/conflict-resolution client surface ADR-0033 implied (0.0.16/0.0.17) is removed, and
  0.0.15 loses the reconcile slice. Per-entity baselines are gone.
- Hand-edits to listing fields reflect after the next boot `stat` pass or an explicit refresh --
  bounded staleness, the trade for running no watcher.
- The orchestrator and its hosts are already async (tokio -- the Tauri commands, the gate/daemon
  bridge, the surface runtime); only the persistence layer (rusqlite/std::fs) is synchronous. An
  async port aligns the data layer with the rest of the crate rather than introducing a runtime;
  the sync fs/sqlite backends wrap via `spawn_blocking`.
- Pairs with ADR-0034: `state.db` holds typed runtime/view state (not the listing cache, which is
  the `fs` adapter's KV layer). ADR-0033's domain-files + operational-store split survives in
  spirit; its merge/baseline/versioning machinery does not.
- The PR #41 snapshot-tree store is reworked into the `fs` `Backend` plus the per-entity stores
  under the layered structure (`entities/` types, `store/` per-entity stores + backends, `app/` use
  cases), not discarded.
