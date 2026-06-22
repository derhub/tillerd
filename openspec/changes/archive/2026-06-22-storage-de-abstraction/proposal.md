## Why

The orchestrator data layer (ADR-0035: `entities/ + infra/ + store/ + app/` over a closed
`Backend { Fs | Sqlite | Memory }` enum, domain entities on a human-readable slug-tree) is
over-abstracted and hard to follow:

- **Storage is fused with domain logic.** `infra/fs` has a file per entity and bakes rules into
  persistence -- `rename_workspace` derives slugs, decides re-slugging, moves directories, reindexes;
  `delete_workspace` hardcodes the Default-guard and reassign cascade.
- **Dead indirection.** The eight `store/<entity>` wrappers are one-line forwarders; the `Backend` enum
  is 629 lines with 22 `wrong_backend` arms; `MemoryBackend` (1119 lines) is a test-only double that
  duplicates the domain rules already in `fs`.
- **A slug-tree for relational data.** Renaming an entity moves its directory, scans for a free slug, and
  reindexes the subtree -- heavy machinery for data that is inherently relational.

## What Changes

- **BREAKING** (internal Rust API + on-disk format, pre-v1): the `store/` module, the `Backend` enum,
  the `FsBackend`/`SqliteBackend` structs, the `infra/memory` backend, and the slug-tree machinery (slug
  derivation, unique-slug scan, directory moves, subtree reindex) are removed.
- **Domain data moves into sqlite via `sqlx`.** All domain entities (workspace, project, session,
  surface, command, launch_template, notification) become typed sqlite rows. Nesting is a `parent_id`
  column; `list` filters and paginates; rename/move/archive are `UPDATE`s; cascades are `UPDATE`/`DELETE`.
  Persistence uses `sqlx` 0.9 (async, compile-time-checked queries; not an ORM) over a `SqlitePool`. The
  slug-tree and its in-memory index are gone.
- **`infra/` is per-entity, entity-aware async repositories.** One repo per entity (`infra/workspace.rs`,
  ...), each owning its table, columns, and `Row -> Entity` mapping, exposing typed
  `create`/`get`/`list(parent, page)`/`update`/`delete`. (The earlier goal of fully entity-agnostic
  storage is dropped: typed columns, ordering, and pagination need entity shape; only the generic
  building blocks below stay agnostic.)
- **`shared/` holds reusable building blocks** (anything used more than once): `fs` (file read/write
  utils for user-config), `kv` (a schemaless `Kv` trait with `SqliteKv` + `MemoryKv` impls, async),
  `datetime`, `errors`, `page` (`Page` + `Listing<T>` for pagination), and the CQS machinery: a generic
  `Command<Cx>`/`Query<Cx>` pair and a `Bus<Cx>` dispatcher. There is no generic storage `Repository`
  trait.
- **`app/` is a CQS layer of command/query objects** named in ubiquitous language (`NewWorkspace`,
  `RenameWorkspace`, `DiscardWorkspace`, `SpawnSurface`, `GetWorkspaceById`, `ListWorkspaces`, ...), each
  implementing `Command<Ctx>` (mutate, returns `()`) or `Query<Ctx>` (read, returns `Out`), async
  `handle(&self, &Ctx)`. A handler reads load -> entity rule -> persist (-> cascade); cross-entity
  cascades live in the parent's command. **The transaction boundary is per command, not the bus** -- a
  command opens one only when it spans multiple writes via `Ctx::transaction(|tx| …)` (commit on `Ok`,
  explicit awaited rollback on `Err`); single writes and runtime-only ops use none. The `Bus<Ctx>` is a
  thin dispatcher carrying telemetry, no transaction. `Ctx` holds the pool, kv, config root, and the
  `Runtime` enum, exposing `db()`/`runtime()`/`transaction()`; repos are executor-passing, so it
  holds no pre-built repo aggregate.
- **`boot/` (composition root)** opens the pool, builds `Ctx` and the thin `Bus<Ctx>`, and injects it.
- **`infra/` is all infrastructure, and the `surface/` and `launch/` dirs are removed.** Their contents
  redistribute into the new layers: `surface/runtime.rs` + `surface/transport.rs` (PTY proxies,
  `daemon_pty_client`, the daemon socket) → `infra/daemon_pty_api/` as a concrete `DaemonPtyApi` (no
  trait); `surface/api.rs` orchestration → `app/` surface commands
  (`SpawnSurface`/`CloseSurface`, launch-on-`NewSession`); `launch/executor.rs` → an `app` command over
  the runtime; `launch/spec.rs` (`LaunchSpec`/`LaunchItem`/`CommandRef` — domain types) → `entities/`.
  `Ctx` holds a `Runtime` enum `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }` (static dispatch). Side
  effects follow D9 (persist intent → effect lock-free → record → reconcile), never inside a
  transaction.
- **Operations use the product's ubiquitous language, not generic CRUD** -- `New*` for creation
  (absorbing the entity draft structs), `Rename*`/`Reorder*`/`MoveProject`/`Archive*`/`Discard*` (hard
  delete)/`SpawnSurface`; reads are descriptive `Get`/`List` (`GetWorkspaceById`, `ListWorkspaces`).
- **User-config stays file-based.** Settings, config, theme, keybindings, and profile are read/written
  through `shared::fs` utils.
- **`entities/` stay pure domain** -- the rules (guards, the rename/title-source rule, the cascade
  policy) live here with no infra trait and no I/O.

- **Transport adapters are thin; the core is transport-agnostic.** The `app/` command/query structs are
  pure (`Serialize`/`Deserialize` + `impl Command`/`Query`, no transport derive). Each transport **owns
  its shim macro in its own layer**: the tauri crate's `transport_command!`/`transport_query!`
  (`type => action`) generates the `#[tauri::command]` shim + `inventory` registration for the operations
  it lists (wire stays `invoke('rename_workspace', { .. })`, Tauri keeps native routing + per-command
  ACL); the future web server crate owns its own macro generating axum routes over the same core types. A
  single dispatch gateway was rejected (it would relocate per-command authorization into app code).
  Wiring splits by ownership: orchestrator `boot::build_bus` builds the core; each transport owns its
  server setup (the tauri app owns `tauri::Builder`).

## Capabilities

### Modified Capabilities

- `store-architecture` -- the per-entity-store-over-`Backend`-enum structure, the `Storage` aggregate,
  the dispatch enum, the in-memory backend, and the slug-tree are removed. Persistence becomes per-entity
  async `sqlx` repositories in `infra/`, with generic building blocks (`fs`/`kv`/`page`/CQS/`bus`) in
  `shared/`; domain data lives in sqlite, user-config in files.
- `app-use-case-layer` -- becomes a CQS layer of command/query objects dispatched through a `Bus`, each
  owning one operation over the repos and entity rules; host controllers stay pure IPC shims.

## Impact

- Code: `crates/orchestrator/src/{entities, shared (new), infra, app, boot}`; `store/`, `infra/memory`,
  the `Backend` enum, the slug-tree machinery, and the `surface/` and `launch/` dirs deleted (contents
  redistributed into entities/infra/app); every caller migrated to the bus.
- Dependencies: add `sqlx` 0.9 (sqlite, async), `tracing-subscriber` (json + env-filter) and
  `tracing-appender` (rolling `*.log`); add a small core-owned proc-macro crate `tillerd-custom-macro`
  hosting the `ErrorCode` derive (reads `#[error_code("…")]` -> `code()`); remove `rusqlite`. The tauri
  transport macro is a declarative `macro_rules!` in the tauri layer. No `opentelemetry`/metrics crates
  (OTel-ready logging, no OTel dependency).
- Behavior: domain on-disk format changes (slug-tree files -> sqlite rows); pre-v1, no released users, so
  the break is accepted with no migration. IPC contract, dynamic ACL, and wire protocol are unchanged.
- Decisions: supersedes ADR-0035 (the store layer, the `Backend` enum, the fs-as-truth-for-domain
  rationale, the slug-tree) via ADR-0036 (de-abstracted storage), ADR-0038 (infra raw API / app owns
  domain), and ADR-0037 (zero-copy event dispatch); commits domain data to sqlite via `sqlx` and the app
  layer to CQS over a bus.
- In scope: the full operation inventory (see design "Operations") across workspace/project/session/
  surface/command/launch-template/template-library/notification + the config plane
  (settings/profile/theme/keybinding incl. the cascade) -- with lifecycle (new/rename/reorder/move/
  archive/restore/discard), pinning, duplicate, fuzzy search, the archive-requires-idle and
  prebuilt-immutable invariants, and the surface I/O channel. `tasks.md` is phased for parallel
  multi-agent execution.
- Out of scope: the broader 0.0.15 feature set (Stronghold secrets, state-model contract); a command
  queue / `Box<dyn Command>` path (additive later if needed).
