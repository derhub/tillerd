# Tasks: storage-de-abstraction

Structured for **parallel multi-agent execution**. Phases run in order; the `[ ]` packages **within a
phase run in parallel** (disjoint files/modules, no shared writes). Each package is one agent's scope.
TDD per package (spec scenario -> red -> green); the final fix-all verify gate is the single barrier.

## Phase 0 -- Foundation (unblocks everything; 0a first, then 0b+0c parallel)

- [x] **0a errors + macro** -- add the `tillerd-custom-macro` crate with the `ErrorCode` derive
  (`#[error_code("…")]` -> `code()`, missing attr = compile error); `shared::Error` enum (`#[from]
  sqlx/io/serde`, `ERROR`-only, no level/category) + `Result` alias. *(blocks all)*
- [x] **0b schema** -- sqlx sqlite schema + migrations for every domain table (workspace/project/session/
  surface/command/launch_template/notification) with `parent_id`, `sort_order`, `pinned`, surface
  `status`, notification `read`/`snooze_until`, archive `status`; add `sqlx` 0.9 dep, drop `rusqlite`.
  *(blocks Phase 2)*
- [x] **0c entities** -- entity rules: guards (`is_default`/`is_unfiled`, **prebuilt-immutable**),
  rename->`title_source`, cascade policy, **archive-requires-idle** predicate, `pinned`; move
  `launch/spec.rs` -> `entities/launch_spec.rs`. Pure unit tests. *(blocks Phase 3)*

## Phase 1 -- shared/ building blocks (parallel; after 0a)

- [x] **1a pagination** -- `Page { All, Offset, Cursor }` + `Listing<T>` + cursor tests
- [x] **1b kv** -- `Kv` trait + `SqliteKv` (sqlx) + `MemoryKv`; round-trip/TTL contract tests on both
- [x] **1c fs** -- atomic read/write/list/delete file utils + tests
- [x] **1d cqs + bus** -- `Command<Cx>`/`Query<Cx>` traits + `Bus<Cx>` (thin `execute<C>`/`query<Q>`,
  telemetry span/error event only, NO tx) + tests
- [x] **1e datetime** -- time helpers

## Phase 2 -- infra/ per-entity repos + runtime (parallel; after 0b, 0c, 1a)

Each repo: typed async sqlx CRUD, **executor-passing** (`impl SqliteExecutor`), `list(parent, page)`
pinned-first, owns `Row -> Entity`; `:memory:` tests (round-trip, parent filter, pagination, update,
delete; a multi-repo call on one tx is atomic).

- [x] **2a** WorkspaceRepo   - [x] **2b** ProjectRepo   - [x] **2c** SessionRepo   - [x] **2d** SurfaceRepo (+ status)
- [x] **2e** CommandRepo   - [x] **2f** LaunchTemplateRepo   - [x] **2g** NotificationRepo (read/snooze)
- [x] **2h runtime** -- concrete `DaemonPtyApi` in `infra/daemon_pty_api/` (from `surface/runtime.rs` +
  `surface/transport.rs`) + `FakeRuntime` test double + `Runtime` enum `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }`;
  raw source exposing `recv() -> Option<SurfaceOutput>` (pull dispatch; no sink held); wire the daemon
  `List` frame (for `ReconcileSurfaces`) and `Stop` (StopSurface, keep record) vs `Kill` (CloseSurface)
  into the client -- both exist in the daemon protocol but not the current client
- [x] **2i config stores** -- `shared::fs`-backed settings/profile/theme/keybinding loaders

## Phase 3 -- app/ commands + queries (parallel per entity; after Phase 2, 1d, 0c)

- [x] **3a context** -- `src/context.rs`: `Ctx { pool, kv, fs_root, runtime: Runtime }` + `db()`/`runtime()`/
  `transaction(|tx| …)` (commit / awaited rollback). *(do FIRST -- blocks 3b+)*
- [x] **3b** workspace ops (New/Rename/Reorder/Archive/Restore/StopWorkspaceSurfaces/Discard/Pin/Unpin; GetById/List)
- [x] **3c** project ops (New/Rename/Reorder/Move/Archive/Restore/StopProjectSurfaces/Duplicate/Discard/Pin/Unpin; GetById/ListByWorkspace/Search; archive cascade)
- [x] **3d** session ops (New/Rename/Reorder/Move/Archive/Restore/StopSessionSurfaces/Duplicate/Discard/Pin/Unpin/ApplyLaunchSpec/ArrangePanels/LaunchSession; GetById/ListByProject/Search/GetLaunchSpec/GetPanelTree; archive-requires-idle; rename->title_source=Custom)
- [x] **3e** surface ops (Spawn/Stop/Close/Detach/ReconcileSurfaces bus commands; input/resize/attach
  off-bus app fns -- attach is lazy per-surface stream bring-up, no eager boot attach-all;
  GetSurfaceById/FindSurfaceByPlacement/ListResumableSurfaces/ListSurfacesBySession queries; D9
  persist-intent -> effect -> record; ReconcileSurfaces converges via daemon `List` (kill orphans, respawn
  missing), no attach; status never logs payloads)
- [x] **3f** command-library ops (New/Rename/Edit/Duplicate/Pin/Discard/Seed; prebuilt-immutable guard)
- [x] **3g** template ops (project-bound `LaunchTemplate` + portable `Template` library, prebuilt guard)
- [x] **3h** notification ops (Record/MarkRead/MarkAllRead/Snooze/Disregard/DisregardAll/Prune; lists/count)
- [x] **3i** config ops (Setting Apply/Reset/Get/List/Resolve/ResolveSettings; Profile/Theme/Keybinding mgmt; ReloadConfig)

Tests: via the `Bus`/`Ctx` over a `:memory:` substrate -- command-mutates-returns-nothing,
query-reads-no-mutation, cascade atomic (Default/Unfiled/prebuilt rejected), archive-idle, side-effect
rollback + reconcile.

## Phase 4 -- transport + boot + cutover (parallel where disjoint; after Phase 3)

- [x] **4a transport macro** -- tauri `transport_command!`/`transport_query!` (`type => action`) ->
  `#[tauri::command]` shims + `inventory` collect; `SurfaceSink` subscriber -> per-surface
  `tauri::ipc::Channel` wiring (pull dispatch via `SurfaceStream`); off-bus input/resize/attach endpoints
- [x] **4b boot** -- `boot::build_bus(cfg) -> Bus<Ctx>`; JSON-lines `tracing-subscriber` + `tracing-appender`
  rolling `*.log` (OTel-named fields, no opentelemetry/metrics crates)
- [x] **4c cutover** -- migrate hosts (`workspace_host`/`surface_host`/`settings_host`/`notification_host`)
  to shims; internals (`surface/api`->app, `surface/runtime`+`surface/transport`->infra/daemon_pty_api,
  `launch/executor`->app, `launch/spec`->entities); **delete** `store/`, `Backend` enum, `infra/memory`,
  slug-tree, `surface/` dir, `launch/` dir. `#[tauri::command]` names + ACL + wire unchanged.
- [x] **4d contract test** -- `command_contract` over a `:memory:` `Ctx`

## Phase 5 -- tests + fix-all verify gate (sequential; last barrier)

- [x] **5a** rewrite implementation-coupled tests to behavior over `:memory:` (drop slug-tree assertions;
  one real path); retain the pre-de-abstraction behavior assertions
- [x] **5b** `bun run verify` (format:check + check-types + lint + test -- unit+integration; e2e excluded)
  green, **then** `bun run e2e` (behavior net) green; confirm no observable IPC/ACL/wire change, no entity
  type in `shared`/`kv`, keystroke payloads never logged

## Phase 6 -- rule-greening (zero ast-grep findings; flip all warnings -> error)

Drives every `.ast-grep/rules` warning to zero, then flips severity to `error`. Dependency order
A->B->C->D keeps the crate compiling at each boundary. New*/repos-take-entities is owned by the
`client-assigned-create-ids` change (its sections 4-5); this phase assumes that has landed.

- [x] **6a entities sqlx derives** -- per VO newtype (`*Id`): `#[derive(sqlx::Type)] #[sqlx(transparent)]`
  (greens `value-object-sqlx-type`, 7). Column enums (`SourceKind`/`SurfaceKind`/`CommandOrigin`/...):
  `#[derive(sqlx::Type)]` + `#[sqlx(rename_all=...)]`; *derived* enums (`ProjectStatus` from `archived_at`)
  stay computed in the SELECT. Each aggregate brace struct: `#[derive(sqlx::FromRow)]` (greens
  `aggregate-entity-fromrow`, 9).
- [x] **6b infra repos query straight into entity** -- drop the `*Row` struct + `From<Row>` map; repos use
  `sqlx::query_as::<_, Entity>` with derived columns via `CASE WHEN ...` in the SELECT. Bind value objects
  directly (transparent). No behavior change.
- [x] **6c queries return Views** -- per query handler: define a flat `*View` (`Serialize` + `sqlx::FromRow`,
  primitive fields, `serde(rename_all="camelCase")`) in the app area; `type Out` becomes
  `Option<View>`/`Vec<View>`/`Listing<View>`/scalar; handler maps straight via `query_as::<_, View>` over
  `cx.db()` -- not repo->entity. Update host/test callsites that consumed the entity. Greens
  `query-returns-view` (29) and the read-path half of `entities-stay-internal`.
- [x] **6d command DTOs hold primitives** -- per Command/Query/Io DTO: fields -> built-in types (`String`
  not `ProjectId`), `pub`; add `#[derive(Deserialize)] #[serde(rename_all="camelCase")]`; convert to value
  objects inside `handle`. Update every test + host callsite (`{ id: ProjectId::new("x") }` ->
  `{ id: "x".into() }`). Greens `message-dto` (77) + `message-dto-deserialize` (85). Largest blast radius.
- [x] **6e boundary ports** -- give the host app-owned, primitive-speaking edges for `infra::runtime`
  (`FakeRuntime`/`Geometry`/`SurfaceEventSink`) and `infra::migrate`, and replace host
  `use orchestrator::entities::{...}` id/enum imports with app-exposed primitives/Views. Greens
  `infra-stays-internal` (4) + the remaining `entities-stay-internal` (5). Nothing deferred.
- [x] **6f flip to error + gate** -- set `severity: error` on all eight transitional rules; `ast-grep scan`
  zero findings, `ast-grep test` snapshots pass, `bun run verify` + `bun run e2e` green.
