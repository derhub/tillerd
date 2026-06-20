## Context

R1a is the structural, behavior-preserving step of the ADR-0035 re-architecture. The orchestrator's
data code is today one oversized, concern-mixing `persistence/` module (`tree/mod.rs` 2559 lines,
`memory.rs` 1067, `mod.rs` 576) with ornamental `DomainStore`/`OperationalStore` traits and a
`CompositeStore` facade that also holds cross-plane logic. The crate runs under tokio (Tauri host,
gate/daemon bridge, surface runtime), but persistence is a **sync island** -- the `Store` trait and
every call site are synchronous today. R1a converts the store API to async (sync backends wrap via
`spawn_blocking`), so call sites across the orchestrator and tauri host gain `.await`. R1a relayers this
into `entities/` + `infra/` + `store/`, with per-entity async stores dispatching over a closed
`Backend` enum, and **no observable behavior change** -- the existing unit suite + e2e stay green.

## Goals / Non-Goals

**Goals:** reviewable single-responsibility modules (no ~1000-line files; tests moved out); entities
separated from storage; one closed `Backend` enum + per-entity store structs replacing the dual
traits + facade; async store API; the fs/sqlite/memory backends working exactly as today behind it.

**Non-Goals (later slices):** the KV listing cache / lazy-load / `mtime` revalidation (R1b); the
`app/` use-case layer (R2) -- R1a parks forced-out coordination at the composition root; surface (R3);
launch (R4); the `postgres` backend; any behavior change.

## Decisions

### D1 -- Module tree

```
crates/orchestrator/src/
  entities/   workspace.rs project.rs session.rs surface.rs  (+ command/setting/notification/launch_template types, ids, enums, New* drafts)
  infra/      fs.rs (+ slug.rs atomic_io.rs index.rs datetime.rs)  sqlite.rs  memory.rs
  store/      backend.rs (enum Backend)  storage.rs (Storage aggregate + create_session coordinator)
              workspaces.rs projects.rs sessions.rs surfaces.rs           (domain)
              commands.rs settings.rs notifications.rs launch_templates.rs (operational)
  <composition root>  builds backends -> stores -> Storage; owns the coordinator fn
```
`persistence/` is deleted. Each file is one responsibility; `#[cfg(test)]` blocks move to sibling
test files or `crates/orchestrator/tests/`. `schema_version` is a small meta/migration fn on the
sqlite backend, not a store.

### D2 -- `Backend` enum (closed, enum dispatch)

`enum Backend { Fs(Arc<FsBackend>), Sqlite(Arc<SqliteBackend>), Memory(Arc<MemoryBackend>) }` in
`store/backend.rs`. Each variant wraps a concrete `infra` backend struct (behind `Arc` so a `Backend`
clones cheaply and a `spawn_blocking` closure can own it). The enum exposes one **async forwarding
method per persisted operation**; each method `match`es the variant and calls the concrete backend's
existing (sync, behavior-preserving) method. `Fs`/`Sqlite` run the blocking call via
`tokio::task::spawn_blocking`; `Memory` runs inline (in-memory, trivially async). Domain operations are
served by `Fs`/`Memory` and operational operations by `Sqlite`/`Memory`; the impossible variant pair
returns a `Persistence` error (never reached given composition-root wiring, see D6). Async fns are
native on the enum -- **no `async-trait`, no `dyn`, no object safety**.

*Why forwarding, not a generic KV primitive:* a `get/put/list(kind,key,filter)` primitive would force
rewriting `FsBackend`'s tree logic (slug/collision, id->path index, `.archive` cascade, layout.json)
into the stores -- a behavior change R1a forbids (Non-Goals; spec "Storage behavior preserved"). The
forwarding enum keeps every backend byte-for-byte (D4) and confines the change to signatures + `.await`.
*Alternative considered:* a generic `Repository<T>` trait (ADR-0035 rejected it -- associated-type +
`async-trait` + object-safety machinery for a closed backend set is the over-abstraction
rust-best-practices warns against).

### D3 -- Per-entity async store structs

One store **per entity**, each `{ backend: Backend }` (or holding a shared `Backend` handle): the
four domain aggregates `Workspaces`, `Projects`, `Sessions`, `Surfaces`, **and** the operational
entities `Commands`, `Settings`, `Notifications`, `LaunchTemplates` (their tables today live behind
`OperationalStore`). Symmetric single-responsibility -- no operational grab-bag survives the
dissolution. `schema_version` is the lone exception: a small meta/migration fn on the sqlite backend,
not an entity store. Methods are the entity's operations -- `get`, `list(&Filter)`, `create`,
`update`, `delete`, plus entity-specifics (`reorder`, archive-representation). `Filter` is a
`#[derive(Default)]` typed per-entity struct (`ProjectFilter { workspace: Option<WorkspaceId>,
include_archived: bool }`) passed to the backend for pushdown. The stores own the domain invariants
that were DB constraints (placement uniqueness, archive-cascade representation), calling the backend.

### D4 -- Map the current tree store onto the `fs` backend (behavior-preserving)

The existing `TreeStore` logic (atomic write-temp-rename, slug + collision, id->path index, `.archive`
subtree move, layout.json bindings, seed) becomes `FsBackend` + the `fs/` helper modules; its current
behavior is preserved byte-for-byte. The current `SqliteStore` (operational tables) becomes
`SqliteBackend`. `InMemoryStore` becomes `MemoryBackend` (one in-memory impl, not a parallel `Store`).
The per-entity stores call these through the `Backend` enum.

### D5 -- Async, with sync backends wrapped

Store + backend methods are `async fn`. fs/sqlite do blocking I/O wrapped in `tokio::task::spawn_blocking`
(or a blocking section) so they don't stall the runtime; memory is trivially async. Call sites run
under tokio and gain `.await`; currently-sync tauri commands become async to call the async stores.

### D6 -- Dissolve the dual traits + facade; `Storage` aggregate at root, narrow deps at leaves

`DomainStore`/`OperationalStore`/`Store`/`CompositeStore` and the single `Arc<dyn Store>` threaded
through ~6 consumer sites are removed. The composition root constructs the backends, builds every
per-entity store with the chosen `Backend` (domain -> `Fs`, operational -> `Sqlite`, tests ->
`Memory`), and bundles them into a plain `Storage` aggregate struct **it owns** (the idiomatic Rust
"AppState" -- a constructor bundle, not a trait, no dispatch). `boot<F: FnOnce() -> Result<Storage>>`;
`Orchestrator { storage: Arc<Storage> }`.

**Leaf consumers depend only on the concrete stores they call**, not the whole bundle
(interface-segregation / least-privilege): `surface_api` <- `Surfaces` + `Sessions`, `launch_executor`
<- `Surfaces` + `Sessions`, `settings_host` <- `Settings`, `notification_host` <- `Notifications`,
`workspace_host` <- `Commands` + `LaunchTemplates` + the coordinator. Threading `Arc<Storage>` deep
would re-create the facade coupling being dissolved (every consumer reaches every store, deps
invisible, tests must build the whole bundle) -- so the aggregate stays at the root only.

*Alternative considered:* a struct-of-stores threaded everywhere -- rejected, it is the dissolved
facade renamed. *Alternative considered:* N concrete Arcs at the root too -- the root genuinely needs
all stores, so bundling there is tidy construction, not coupling.

### D7 -- `create_session` cross-aggregate coordination is a standalone coordinator, not a store method

Resolving a `launch_template` -> spec then materializing a `Session` spans two aggregates. By DDD
layering it is **application work, not repository work**:

- NOT a method on the `Sessions` store -- a per-entity store must not depend on `LaunchTemplates`
  (single-responsibility leak; reintroduces cross-aggregate coupling into infra).
- NOT in the tauri hosts -- coordination in controllers is the anemic-domain anti-pattern, and it is
  duplicated across `workspace_host` + `surface_api`.
- A standalone coordinator fn (`create_session(draft, &LaunchTemplates, &Sessions) -> Session`) the
  composition root owns; both real callers delegate to it.

R1a parks the coordinator at the root; R2 promotes it verbatim into the `app/` use-case layer. This
is exactly what the proposal means by "forced out to the composition root in R1a, formalized into
`app/` in R2" -- behavior-identical here. Aligns with `rust-best-practices` (no god-object, narrow
ownership) and DDD layering.

### D8 -- Tests relocate, assertions unchanged

Each module's `#[cfg(test)]` moves to a sibling file; contract-level store tests that reach the public
API move to `crates/orchestrator/tests/`. Assertions are unchanged or only adapted to async (`.await`).
The 113 unit tests + the e2e suite must stay green -- that green-and-unchanged result is the proof the
refactor preserved behavior.

## Risks / Trade-offs

- **Enum dispatch means add-a-backend edits the enum + matches** -> acceptable for a closed set; a
  trait is introduced only if open/plugin backends ever appear (ADR-0035).
- **Async conversion blast radius** -> the current `Store` trait is **sync**; the per-entity stores
  expose async (sync backends wrap blocking work via `spawn_blocking`), so every store call site
  across the orchestrator **and** the tauri host gains `.await`, and currently-sync tauri commands
  (`workspace_host`/`settings_host`/`notification_host`) become async. Mechanical but wide. Mitigated
  by doing it as a pure signature change with no logic edits, phased per module, suite green between.
- **Large diff (relayering the whole data module)** -> mitigated by R1a being structural-only (R1b/R2+
  split off) and by tasks phased per module so review is per-file.
- **Behavior drift hidden in the move** -> the unchanged test suite is the guard; any test needing a
  real edit (beyond `.await`) is a signal the move changed behavior -- stop and fix.

## Migration Plan

Incremental, keeping `cargo test` green between steps: introduce `entities/` (move types) -> `infra/`
(move backends, rename `tree`->`fs`, extract helpers, move tests) -> `store/` (`Backend` enum +
per-entity stores) -> rewire composition root + delete the dual traits/facade -> relocate remaining
tests. Pre-v1, no data migration. Rollback: revert the branch.

## Open Questions

- None blocking. Deferred by design: KV cache/lazy/mtime (R1b), `app/` extraction (R2), `postgres`
  backend, exact `Backend` low-level primitive signature (settled in implementation, kept minimal).
