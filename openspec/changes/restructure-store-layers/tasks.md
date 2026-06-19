## 1. `entities/` -- extract types

- [ ] 1.1 Move the domain types out of `persistence/mod.rs` into `entities/` (one module per
  aggregate: `workspace`/`project`/`session`/`surface`) -- structs, newtype ids, enums, `New*`
  drafts; pure, no infra deps. Update imports; keep `cargo build` green.

## 2. `infra/` -- backends as concrete structs

- [ ] 2.1 Move the backends out of `persistence/` into `infra/`: rename the tree store -> `fs.rs`
  (`FsBackend`) and extract its plumbing to sibling helpers (`slug.rs`, `atomic_io.rs`, `index.rs`,
  `datetime.rs`); `sqlite.rs` (`SqliteBackend`); `memory.rs` (`MemoryBackend` -- one in-memory impl,
  not a parallel `Store`). Preserve current behavior exactly. Move each module's `#[cfg(test)]` into
  sibling test files (or `crates/orchestrator/tests/` for contract-level).

## 3. `store/` -- `Backend` enum + per-entity async stores

- [ ] 3.1 `store/backend.rs`: closed `enum Backend { Fs(FsBackend), Sqlite(SqliteBackend),
  Memory(MemoryBackend) }` exposing the low-level async storage primitive (dispatch by `match`); sync
  backends wrap blocking work via `spawn_blocking`.
- [ ] 3.2 One per-entity async store struct per aggregate (`Workspaces`/`Projects`/`Sessions`/
  `Surfaces`) holding a `Backend`, exposing `get / list(&Filter) / create / update / delete` +
  entity-specifics (reorder, archive-representation), with `#[derive(Default)]` typed per-entity
  `Filter` structs pushed to the backend. The stores own the invariants that were DB constraints
  (placement uniqueness, archive cascade).

## 4. Wire-up -- dissolve the facade

- [ ] 4.1 Delete `DomainStore`/`OperationalStore`/`Store`/`CompositeStore` and the `persistence/`
  module; rewire the composition root to construct backends, build the per-entity stores with the
  chosen `Backend` (domain->`Fs`, operational->`Sqlite`, tests->`Memory`), and host `create_session`'s
  template resolution as a thin composition fn (formalized to `app/` in R2). Update call sites to the
  async store API (`.await`).

## 5. Verify gate (fix-all)

- [ ] 5.1 Run the full verify suite to green: `fmt`, `clippy --all-targets --locked -D warnings`,
  unit tests (relocated/adapted, all green -- incl. the `store-architecture` scenarios: cross-backend
  round-trip parity, composition-root selection, behavior preserved), and the affected e2e
  (boot/create/resume). No module left ~1000 lines; assertions unchanged beyond `.await`. Fix any
  gap and re-run until clean.
