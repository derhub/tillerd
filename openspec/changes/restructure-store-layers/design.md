## Context

R1a is the structural, behavior-preserving step of the ADR-0035 re-architecture. The orchestrator's
data code is today one oversized, concern-mixing `persistence/` module (`tree/mod.rs` 2559 lines,
`memory.rs` 1067, `mod.rs` 576) with ornamental `DomainStore`/`OperationalStore` traits and a
`CompositeStore` facade that also holds cross-plane logic. The crate is already async (tokio -- Tauri
host, gate/daemon bridge, surface runtime); persistence is the lone sync island. R1a relayers this
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
  entities/   workspace.rs project.rs session.rs surface.rs  (+ ids, enums, New* drafts)
  infra/      fs.rs (+ slug.rs atomic_io.rs index.rs datetime.rs)  sqlite.rs  memory.rs
  store/      backend.rs (enum Backend) workspaces.rs projects.rs sessions.rs surfaces.rs
  <composition root>  wires Backend per entity; hosts forced-out coordination
```
`persistence/` is deleted. Each file is one responsibility; `#[cfg(test)]` blocks move to sibling
test files or `crates/orchestrator/tests/`.

### D2 -- `Backend` enum (closed, enum dispatch)

`enum Backend { Fs(FsBackend), Sqlite(SqliteBackend), Memory(MemoryBackend) }` in `store/backend.rs`.
Each variant is a concrete `infra` backend struct. The enum exposes the low-level storage primitive
the per-entity stores call (read/write/list a record by kind + key + filter); each method `match`es
the variant. Async fns are native on the concrete enum/structs -- **no `async-trait`, no `dyn`, no
object safety**. *Alternative considered:* a generic `Repository<T>` trait (ADR-0035 rejected it --
associated-type + `async-trait` + object-safety machinery for a closed backend set is the
over-abstraction rust-best-practices warns against).

### D3 -- Per-entity async store structs

One per aggregate: `Workspaces`, `Projects`, `Sessions`, `Surfaces`, each `{ backend: Backend }` (or
holding a shared `Backend` handle). Methods are the entity's operations -- `get`, `list(&Filter)`,
`create`, `update`, `delete`, plus entity-specifics (`reorder`, archive-representation). `Filter` is
a `#[derive(Default)]` typed per-entity struct (`ProjectFilter { workspace: Option<WorkspaceId>,
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
(or a blocking section) so they don't stall the runtime; memory is trivially async. Call sites already
run under tokio, so they `.await` naturally.

### D6 -- Dissolve the dual traits + facade; composition root wires backends

`DomainStore`/`OperationalStore`/`Store`/`CompositeStore` are removed. The composition root constructs
the backends, builds the per-entity stores with the chosen `Backend` (domain -> `Fs`, operational ->
`Sqlite`, tests -> `Memory`), and exposes them to the orchestrator. `create_session`'s template
resolution (was in `CompositeStore`) lands here as a thin composition fn, formalized into `app/` in R2.

### D7 -- Tests relocate, assertions unchanged

Each module's `#[cfg(test)]` moves to a sibling file; contract-level store tests that reach the public
API move to `crates/orchestrator/tests/`. Assertions are unchanged or only adapted to async (`.await`).
The 113 unit tests + the e2e suite must stay green -- that green-and-unchanged result is the proof the
refactor preserved behavior.

## Risks / Trade-offs

- **Enum dispatch means add-a-backend edits the enum + matches** -> acceptable for a closed set; a
  trait is introduced only if open/plugin backends ever appear (ADR-0035).
- **Async signature churn across call sites** -> mechanical; the crate is already async, so callers
  `.await`. Mitigated by doing it as a pure signature change with no logic edits.
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
