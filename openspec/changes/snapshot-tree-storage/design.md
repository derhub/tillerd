## Context

The orchestrator persists everything in one SQLite file (`tillerd.db`) via a single `Store`
trait (`crates/orchestrator/src/persistence/{mod.rs,schema.rs,sqlite.rs,memory.rs}`), migrations
v1–v7. ADR-0033 splits persistence into a readable domain plane (JSON snapshot tree) and an
operational plane (SQLite). This slice lands the domain plane and removes the worktree entity;
the operational plane stays in SQLite (formalized as `state.db` in slice 2). Pre-v1, dev-only
data is discarded (clean cutover). This supersedes the ADR-0023 data-model freeze (authorized by
ADR-0033/0034). Constraints: single embedded orchestrator process; no new crate
([[crate-layout-preference]]); no file watchers; `std`-only file I/O + `serde_json` (already in
tree).

## Goals / Non-Goals

**Goals:**

- Domain (`workspace → project → session` + `layout.json`) persists as a JSON file tree under a
  relocatable data-root directory; the file is the live truth.
- The worktree entity and `git worktree add` provisioning are gone; a working directory is `cwd`.
- The orchestrator's request/response API signatures for domain CRUD stay stable so the UI is
  unaffected beyond the `SourceKind` type.
- Invariants SQLite enforced implicitly (placement uniqueness, write serialization) move to
  store code, explicitly.

**Non-Goals (later slices):** operational `state.db` final typed schema + persisted id→path
index + baselines (slice 2); state-model contract / lifecycle FSM / guards / sync status (slice
2); settings profiles + cascade (slice 3); Stronghold secrets (slice 3); reconcile —
startup 2-way / Re-sync 3-way `merge3`, full malformed-file resilience, conflict prompts (slice
4). No file watchers in any slice.

## Decisions

### D1 — Split `Store` into `DomainStore` (files) + `OperationalStore` (SQLite); compose behind a facade

Two traits in `persistence/`: `DomainStore` (workspace/project/session/surface-binding CRUD,
list, reorder, archive, layout) backed by a new file-tree impl; `OperationalStore`
(meta/command/setting/notification/launch_template) backed by the existing `SqliteStore`. A thin
`Store` facade holds both and delegates, so most call sites keep one injected handle.
*Alternative:* one trait, file-or-SQLite per method — rejected: muddies the plane boundary the
ADR draws and blocks slice 2's `state.db` swap. Operational tables drop FK clauses to domain ids
(ids become opaque text keys validated against the index).

### D2 — File-tree store module: `persistence/tree/`

New module (not a crate). Layout under the data root:
`workspaces/<ws-slug>/workspace.json`, `.../projects/<proj-slug>/project.json`,
`.../sessions/<sess-slug>/{session.json,layout.json}`, archived subtrees under a sibling
`.archive/`. Entity JSON carries the stable `id` + `sortOrder` + entity fields; no parent-id
field (containment encodes hierarchy). Serialize with `serde_json` (pretty, trailing newline)
reusing the existing domain structs minus the dropped fields.

### D3 — In-memory `RwLock<TreeState>` holding the id→path index; boot scan builds it

`TreeState { index: HashMap<EntityId, PathBuf>, ... }` behind a `RwLock`. At boot, walk the tree
once and populate the index; `get_*` resolves id→path through it. Every create/rename/archive/
delete updates the index under the write lock — this is also the **in-process write
serialization** (D1's replacement for SQLite's connection mutex). Persisting the index is slice
2; here it is always scan-rebuilt.

### D4 — Atomic writes via write-temp-rename; archive/rename via `fs::rename`

Write `<file>.tmp` then `rename` into place (atomic on same fs). `create_dir_all` for new entity
dirs. Archive moves the entity directory (whole subtree) to `.archive/<slug>/` with one
`fs::rename`; this *is* the cascade (sessions move with their project). Hard-delete removes the
archived subtree. *Alternative:* per-entity `deleted_at` flag in-file — rejected: keeps dead
entities in the live tree, complicates listing and diffing.

### D5 — Slug derivation + collision suffixing; rename = re-slug + subtree move

Slug = lowercase, non-alphanumerics → `-`, collapsed/trimmed; empty → the id's short form. On
create/rename, if a sibling slug exists, suffix `-2`, `-3`, … The stable `id` never changes;
rename moves the directory (subtree) and updates the index. Slug is cosmetic; the id is truth.

### D6 — Placement uniqueness enforced in store code

Before binding a surface to a `(session, placement)`, the store checks the session's `layout.json`
bindings under the write lock and rejects a collision with the existing `SurfaceConflict` error
(replacing the v4 `surface_session_placement` partial-unique index).

### D7 — Operational schema collapsed to operational tables only (clean cutover)

Rewrite the SQLite schema to a single operational migration (meta/command/setting/notification/
launch_template) — the domain DDL (`workspace`/`project`/`session`/`surface`) and `worktree` are
removed, not migrated, since dev-only data is discarded. Operational FK-to-domain clauses are
dropped. The file keeps the `tillerd.db` name this slice; renamed to `state.db` in slice 2.

### D8 — Data-root path

`tillerd_paths` gains a data-root **directory** builder (default `<tillerd_dir>/data`), under
which `workspaces/` lives; the old single-file product-store path is removed. A user-configurable
data root via `config.jsonc` is slice 3 — this slice uses the fixed default.

### D9 — Seed on empty tree

First boot with an empty tree seeds the Default workspace (fixed id) and the Unfiled project
(fixed id, `blank`) as directories, preserving the prior seeded-defaults behavior.

### D10 — Testing: tempdir-backed `DomainStore`

Domain unit tests construct the file-tree store over a `TempDir` (one fixture per scenario,
isolated, order-independent), mirroring the role `InMemoryStore` played. `InMemoryStore` is
retained only for the operational trait (or dropped if unused). Each spec `#### Scenario:` maps
1:1 to a unit test (red→green→refactor).

## Risks / Trade-offs

- **Malformed/partial domain file at boot crashes the scan** → slice-1 minimal handling: skip
  mounting that entity and log; full per-class fallback + notification + reconcile is slice 4.
  Documented as a deliberate minimal gap, not silent.
- **Cross-filesystem `rename` fails (data root on another mount)** → data root is a single
  directory; all moves are intra-root, so `rename` stays atomic. If a user repoints the data root
  (slice 3), that slice owns cross-fs handling.
- **Index drift if a mutation updates files but not the index** → all mutations go through the
  single write-locked path that updates files and index together; no other writer exists
  in-process.
- **API-signature drift forcing UI changes** → domain CRUD method signatures are preserved;
  only `SourceKind` (drop `git_worktree`) and surface shape (drop `worktree_id`) change, both
  already breaking per the proposal.
- **Big diff across persistence + launch + specs** → mitigated by doing the worktree drop first
  (isolated, mechanical) before the store rewrite, per the task order.

## Migration Plan

Clean cutover (pre-v1): no data migration. On deploy, an existing `tillerd.db` is ignored for
domain data (domain tables no longer read); first boot finds an empty tree and seeds Default +
Unfiled. Rollback: pre-v1, none — revert the branch. Mark breaking commits `!` / `BREAKING
CHANGE`.

## Open Questions

- None blocking. Deferred-by-design: persisted id→path index + `state.db` rename (slice 2),
  configurable data root (slice 3), full malformed-file/reconcile resilience (slice 4).
