# app-use-case-layer Specification

## Purpose
App-layer use-case boundary between host controllers and the domain. Covers session creation with launch-template resolution, CQS command/query dispatch through a Bus over a lazy Ctx, per-command transaction scoping, side effects persisted as intent and reconciled on boot, transport-agnostic operations with generated per-transport shims, and lifecycle operations (archive/restore/duplicate/move, notifications, templates, config plane, session layout planes).
## Requirements
### Requirement: Session creation resolves a launch template

The app layer SHALL own session creation: given a draft, it resolves the draft's launch
template (if any) into a concrete spec and materializes the session. This coordination spans the
`LaunchTemplates` and `Sessions` aggregates and SHALL NOT live in a store method or a host
controller.

#### Scenario: Draft with a template id materializes a session carrying the instantiated spec

- **WHEN** `create_session` is called with a draft whose `template_id` references an existing template
- **THEN** the template is resolved, its spec is instantiated for the session, and the persisted session carries that spec at the template's spec version

#### Scenario: Draft without a template id materializes a session with no spec

- **WHEN** `create_session` is called with a draft whose `template_id` is `None`
- **THEN** the session is persisted with no launch spec and no template lookup occurs

#### Scenario: Draft referencing an unknown template fails

- **WHEN** `create_session` is called with a `template_id` that does not exist
- **THEN** it returns a `LaunchTemplateNotFound` error and no session is persisted

### Requirement: Opening a session creates then activates it

The app layer SHALL expose an `open_session` use case that sequences create-then-activate:
materialize the session, then activate its surfaces best-effort through a narrow activation port.
Activation is decoupled from the concrete surface runtime so the use case is host-agnostic.

#### Scenario: open_session persists the session and activates its surfaces

- **WHEN** `open_session` is called with a draft and an activator
- **THEN** the session is persisted and the activator is invoked once with the new session's id

#### Scenario: Activation failure is non-fatal

- **WHEN** `open_session` is called and the activator returns an error
- **THEN** the failure is logged and the created session is still returned successfully

### Requirement: Hosts delegate cross-aggregate coordination to the app layer

Host controllers SHALL be pure IPC shims: they map their transport request into a draft and
delegate to the app use case, never assembling the create-then-activate sequence themselves.

#### Scenario: The desktop session-create command delegates to the app use case

- **WHEN** the tauri `session_create` command runs
- **THEN** it builds a draft from its arguments, calls `open_session`, and returns the created session — the command contains no cross-aggregate sequencing of its own

### Requirement: Operations are CQS command/query objects

The app layer SHALL express every operation as a value type implementing one of two generic contracts
(defined in `shared/`): a `Command<Cx>` for mutations (async `handle(&self, &Cx) -> Result<()>`,
returning no value) or a `Query<Cx>` for reads (async `handle(&self, &Cx) -> Result<Self::Out>`). A
command SHALL NOT return data and a query SHALL NOT mutate state (Meyer's command-query separation).
There SHALL be one operation type per case, grouped per entity. Operation names SHALL use the product's
ubiquitous language, not generic mutation CRUD: creation is `New*` (`NewWorkspace`, absorbing the
entity draft struct), plus `Rename*`/`Reorder*`/`MoveProject`/`Archive*`/`Discard*` (hard-delete)/
`SpawnSurface`/`CloseSurface`/`ApplyLaunchSpec`/`ArrangePanels`; read queries are descriptive `Get`/`List`
(`GetWorkspaceById`, `ListWorkspaces`, `ListProjectsByWorkspace`). Each command handler SHALL read
top-to-bottom: load the entity, apply the entity method, persist through the repository (and cascade
where required). Cross-entity cascades SHALL live in the parent's command.

#### Scenario: A command mutates and returns nothing

- **WHEN** a rename/move/archive/delete command's `handle` runs
- **THEN** it loads the entity, applies the entity rule, persists via the repository, and returns
  `Result<()>` with no returned data

#### Scenario: A query reads and does not mutate

- **WHEN** a get/list query's `handle` runs
- **THEN** it returns its `Out` (e.g. `Option<Workspace>`, `Listing<Project>`) and performs no write

#### Scenario: Archive is rejected unless all in-scope sessions are idle

- **WHEN** `ArchiveSession`/`ArchiveProject`/`ArchiveWorkspace` runs while any session in scope has a
  running (`live`) surface
- **THEN** the command is rejected by the archive-requires-idle entity rule and nothing is archived
- **WHEN** every session in scope is idle (no `live` surfaces)
- **THEN** the archive proceeds and cascades to children

#### Scenario: Prebuilt commands and templates are immutable

- **WHEN** `RenameCommand`/`EditCommand`/`DiscardCommand` (or `DiscardTemplate`) targets a `Prebuilt` item
- **THEN** the command is rejected by the prebuilt-is-immutable guard; only `Custom` items can be renamed,
  edited, or discarded (`DuplicateCommand` makes an editable custom copy of a prebuilt)

#### Scenario: Deleting a workspace cascades from the parent's command

- **WHEN** `DiscardWorkspace` runs for a non-default workspace
- **THEN** it reassigns the workspace's projects to Default (via `cx.projects()`) and then deletes the
  workspace, and deleting the Default workspace is rejected by the entity guard

#### Scenario: Renaming a session marks its title as custom

- **WHEN** `RenameSession` runs
- **THEN** the session's title is updated and its `title_source` becomes `Custom`, so later automatic
  titling does not override the user's name

### Requirement: Commands and queries are dispatched through a Bus over a lazy context

A generic `Bus<Cx>` SHALL be the single dispatch point, exposing `execute<C: Command<Cx>>` and
`query<Q: Query<Cx>>` (static dispatch); it carries cross-cutting telemetry but SHALL NOT own a
transaction. The context `Ctx` SHALL hold only real resources (the `SqlitePool`, the `SqliteKv`, the
config root, and a `Runtime` enum `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }`), expose the pool
(`db()`), the runtime (`runtime()`), and an opt-in `transaction(|tx| …)` helper, and SHALL NOT hold a
pre-built repository aggregate. The composition root (`boot`) SHALL build `Ctx` and `Bus<Ctx>` and inject
the bus.

#### Scenario: An operation is dispatched through the bus

- **WHEN** a caller runs an operation
- **THEN** it constructs the command/query value and calls `bus.execute(..)` or `bus.query(..)`

#### Scenario: No boxed dispatch is introduced

- **WHEN** the bus dispatches an operation
- **THEN** it uses the concrete operation type (static dispatch); there is no `Box<dyn Command>` path
  unless a command queue is later added

### Requirement: The transaction boundary is per command, not on the bus

A command SHALL open a transaction **only when it spans multiple writes**, via `Ctx::transaction(|tx| …)`,
which SHALL commit on `Ok` and **explicitly, awaited-roll-back on `Err`** (not left to the transaction's
`Drop`); a failed rollback SHALL be logged while the original error propagates. A single-statement
command SHALL use the pool directly (one statement is atomic). A runtime-only or side-effecting command
(e.g. surface input/resize, or a spawn that drives the runtime) SHALL NOT be wrapped in a database
transaction. Repository methods SHALL take a sqlx executor (a pool reference or a transaction reference),
so the same method serves both a direct call and a transactional one. A command SHALL NOT re-dispatch
through the bus (which would nest transactions). A query SHALL be read-only and use no transaction.

#### Scenario: A multi-repo cascade is atomic

- **WHEN** a command mutates more than one repository (e.g. discard-workspace reassigns projects then
  deletes the workspace)
- **THEN** it wraps those writes in `Ctx::transaction`, and if any step fails the transaction rolls back
  so none of them persist

#### Scenario: A single-write command uses no transaction

- **WHEN** a command performs exactly one write (e.g. rename)
- **THEN** it calls the repository on the pool directly, opening no explicit transaction

#### Scenario: A runtime/side-effecting command is not DB-transaction-wrapped

- **WHEN** a command drives the runtime port (e.g. spawn/stop/close a surface)
- **THEN** it performs its persistence (if any) and its runtime side effect without a surrounding database
  transaction

#### Scenario: Surface input/resize/attach are an I/O channel, not commands

- **WHEN** input, resize, or attach (connect the proxy stream) is sent to a surface
- **THEN** it goes host -> an `app` direct function -> `Runtime`/`DaemonPtyApi` methods, skipping the
  **bus** (no command object, no telemetry) but still through `app` (the host never calls infra directly);
  and no span, event, or metric captures the input payload. (`detach` is a regular bus command.)

### Requirement: Side effects run outside the transaction; intent is persisted and reconciled

A command with an external side effect (spawn a PTY, launch a process, a network call) SHALL NOT run that
effect inside a database transaction. It SHALL: (1) persist the desired state in a short, committed write
(e.g. a surface at status `pending`); (2) run the effect lock-free via the runtime; (3) record the
outcome in a second short write (`live`/`failed`); and (4) rely on a boot reconciler (`ReconcileSurfaces`)
to converge actual runtime state to the persisted desired state on boot and after failures. The reconciler
SHALL enumerate live daemon PTYs (via the daemon `List` frame) and converge -- desired-but-not-running ->
respawn or mark failed; running-but-no-row -> kill -- and SHALL NOT attach proxy streams (streaming is
brought up lazily per surface by `AttachSurface` when a renderer registers its Channel). The DB is the
source of truth for intent; compensation-only handling (without a reconciler) SHALL NOT be the recovery
mechanism, as it is not crash-safe.

#### Scenario: A spawn never holds the write lock across the effect

- **WHEN** a surface is spawned
- **THEN** the desired-state row is committed before the spawn, the spawn runs with no transaction held,
  and the outcome status is written after -- the sqlite write lock is not held during the spawn

#### Scenario: A crash mid-effect is reconciled, not stranded

- **WHEN** the process crashes after the desired state is persisted but before the effect completes
- **THEN** on next boot the reconciler observes desired-but-not-running and converges (respawn or mark
  failed), with no reliance on a compensating write having run

#### Scenario: Boot reconcile converges without attaching streams

- **WHEN** `ReconcileSurfaces` runs at boot
- **THEN** it enumerates live daemon PTYs via the `List` frame, kills any PTY with no desired row, and
  respawns/marks any desired row with no live PTY, and it attaches no proxy stream (no Channel is required
  and no scrollback is replayed -- that happens later, lazily, on `AttachSurface`)

#### Scenario: Launching a session instantiates its spec onto the runtime

- **WHEN** `LaunchSession` runs for a session with a launch spec
- **THEN** each spec item's surface is spawned onto the runtime following the side-effect shape (persist
  intent -> spawn lock-free -> record outcome), and a session with no spec launches nothing

### Requirement: The command/query core is transport-agnostic

The `Command`/`Query` types and the `Bus` SHALL carry no transport knowledge (no tauri or HTTP types).
Each transport SHALL be a thin adapter that builds a command/query value and calls `bus.execute`/
`bus.query`. The same commands and bus SHALL be reusable by a future web server adapter without change.

#### Scenario: A command runs the same way regardless of transport

- **WHEN** an operation is invoked from tauri or (later) from an HTTP route
- **THEN** the same `Command`/`Query` type and the same `bus` are used, and only the thin transport
  adapter differs

### Requirement: Each transport owns its shim generation; the tauri shims are generated, not hand-written

The core command/query structs SHALL carry no transport-specific derive; transport shim generation SHALL
live in each transport's own layer. The tauri layer SHALL own a macro (`transport_command!` /
`transport_query!`, `type => action`) that lists the operations it exposes and generates, per operation,
a `#[tauri::command]` shim whose name is the action (wire stays `invoke('rename_workspace', { ... })`) and
registers it (e.g. via `inventory`), so Tauri keeps native per-command routing, argument typing, and
per-command ACL/capabilities. There SHALL be no single dispatch gateway that relocates per-command
authorization into app code. A future server transport SHALL own its own macro (axum routes) over the
same core types. The tauri transport macro SHALL be a declarative macro in the tauri layer (not a
proc-macro crate). Existing tauri command names, the dynamic ACL, and the wire protocol SHALL be
unchanged.

#### Scenario: A generated shim delegates to the bus

- **WHEN** a tauri rename/archive/reorder/move/get/list command runs
- **THEN** the generated shim builds the command/query value, calls `bus.execute`/`bus.query`, returns
  the result with no logic of its own, and its command name is unchanged

#### Scenario: Adding an operation needs no hand-written shim

- **WHEN** a new core operation type is added with its `impl Command`/`Query` and listed via the tauri
  layer's `transport_command!`/`transport_query!`
- **THEN** its tauri command shim and registration are generated, with no hand-written shim and no manual
  `generate_handler!` edit, and the core struct gains no transport dependency

### Requirement: Archive is reversible via Restore

Every archivable entity (workspace, project, session) SHALL have a `Restore*` command that returns an
archived entity to active state. Restoring SHALL reactivate the entity; whether children auto-restore
SHALL follow the cascade policy in `entities/`.

#### Scenario: An archived entity is restored

- **WHEN** `RestoreWorkspace`/`RestoreProject`/`RestoreSession` runs on an archived entity
- **THEN** it becomes active again and reappears in its parent's live listing

#### Scenario: Restore targets only archived entities

- **WHEN** a `Restore*` command runs on an entity that is not archived
- **THEN** it is rejected (already active) and nothing changes

### Requirement: Entities can be duplicated

`DuplicateProject` SHALL clone a project with its sessions and launch specs; `DuplicateSession` SHALL
clone a session with its launch spec; `DuplicateProfile` SHALL copy a profile under a new name;
`DuplicateCommand` SHALL make an editable `Custom` copy of any command including a `Prebuilt`. A duplicate
SHALL be independent of its source.

#### Scenario: Duplicating clones the subtree independently

- **WHEN** `DuplicateProject`/`DuplicateSession` runs
- **THEN** a new entity with copies of the source's children/specs is created, and mutating the copy does
  not affect the source

#### Scenario: Duplicating a prebuilt yields an editable custom

- **WHEN** `DuplicateCommand` targets a `Prebuilt`
- **THEN** the copy is `Custom` and can be renamed/edited/discarded

### Requirement: Move reassigns an entity's parent

`MoveProject` SHALL reassign a project to another workspace and `MoveSession` SHALL reassign a session to
another project, by updating the `parent_id` with no directory move.

#### Scenario: A move reparents by update

- **WHEN** `MoveProject`/`MoveSession` runs
- **THEN** the entity's `parent_id` is updated, it appears under the new parent's listing and no longer
  under the old, with no directory move or slug scan

### Requirement: Surfaces can be stopped in bulk to make a scope idle

`StopSessionSurfaces`, `StopProjectSurfaces`, and `StopWorkspaceSurfaces` SHALL stop every running
(`live`) surface in their scope so the scope becomes idle (the precondition for archive). Each SHALL stop
surfaces via the surface stop path, with no database transaction held across the runtime effect (D9).

#### Scenario: Stopping a scope makes it idle

- **WHEN** `StopProjectSurfaces` runs
- **THEN** every live surface under the project is stopped and the project's sessions become idle, so a
  subsequent `ArchiveProject` is permitted

### Requirement: Notification center operations

The notification operations SHALL be `RecordNotification`, `MarkNotificationRead`,
`MarkAllNotificationsRead`, `SnoozeNotification` (set `snooze_until`), `DisregardNotification`,
`DisregardAllNotifications`, and `PruneNotifications` (retention cap), with queries `ListNotifications`,
`ListUnreadNotifications`, and `CountUnreadNotifications`.

#### Scenario: Marking read clears the unread badge

- **WHEN** `MarkNotificationRead`/`MarkAllNotificationsRead` runs
- **THEN** the marked records leave `ListUnreadNotifications` and `CountUnreadNotifications` drops
  accordingly

#### Scenario: Snooze defers a notification

- **WHEN** `SnoozeNotification` sets a future `snooze_until`
- **THEN** the record is suppressed from the active list until that time passes

#### Scenario: Disregard removes a notification

- **WHEN** `DisregardNotification`/`DisregardAllNotifications` runs
- **THEN** the targeted record(s) are deleted

### Requirement: Template operations cover project launch templates and the portable library

Project launch templates SHALL have `NewLaunchTemplate`/`ApplyTemplateSpec`/`DiscardLaunchTemplate` with
queries `GetLaunchTemplateById`/`ListLaunchTemplatesByProject`. The portable template library SHALL have
`ImportTemplate`/`ExportTemplate`/`DiscardTemplate` and `Pin*`/`Unpin*`, with queries
`ListTemplates`/`GetTemplateById`. A `Prebuilt` library template SHALL reject discard/edit.

#### Scenario: A project launch template is created and replaced

- **WHEN** `NewLaunchTemplate` then `ApplyTemplateSpec` runs for a project
- **THEN** the project's saved launch spec is set and then replaced

#### Scenario: A prebuilt library template is immutable

- **WHEN** `DiscardTemplate` targets a `Prebuilt` library template
- **THEN** it is rejected by the prebuilt-immutable guard; only `Custom` templates can be discarded

### Requirement: Config plane lifecycle operations

Beyond settings, the config plane SHALL expose profile operations (`NewProfile`/`RenameProfile`/
`DuplicateProfile`/`DiscardProfile`/`ActivateProfile`/`ImportProfile`/`ExportProfile`, queries
`ListProfiles`/`GetActiveProfile`), theme operations (`ActivateTheme`/`ImportTheme`/`DiscardTheme`/
`ExportTheme`, queries `ListThemes`/`GetActiveTheme`), and keybinding operations (`RebindKey`/
`ResetKeybinding`/`ResetKeybindings`, queries `ListKeybindings`/`ResolveKeybinding`), all persisted via
`shared::fs`. Activating a profile SHALL drive the settings cascade.

#### Scenario: Activating a profile changes the effective settings

- **WHEN** `ActivateProfile` switches the active profile
- **THEN** `ResolveSetting`/`ResolveSettings` reflect the new profile's values through the cascade

#### Scenario: A rebind persists and resolves

- **WHEN** `RebindKey` sets an action -> chord
- **THEN** `ListKeybindings`/`ResolveKeybinding` reflect it, and `ResetKeybinding` reverts that binding to
  its default

### Requirement: The bus emits OTel-ready structured logs

The bus SHALL be the single instrumentation point: one span per operation and, on error, one structured
`ERROR` event carrying OTel-named fields (`error.type` = the error's `code()`, `exception.message`,
`source`, `trace_id`). Logs SHALL be emitted as JSON lines via `tracing-subscriber` (json + env-filter)
to a rolling `*.log` via `tracing-appender`, with no `opentelemetry` or metrics crates. A surface input
payload SHALL NOT appear in any span, event, or metric.

#### Scenario: An operation error logs one structured ERROR event

- **WHEN** a command/query returns `Err`
- **THEN** the bus logs exactly one `ERROR` event whose `error.type` is the stable `code()` and whose
  `exception.message`/`source` carry the cause, serialized as a JSON line

#### Scenario: Keystroke payloads never reach the log

- **WHEN** surface input flows through the I/O channel (off the bus)
- **THEN** no log line, span, event, or metric contains the input bytes

### Requirement: A session's layout is a launch spec and a panel-tree geometry, joined by surface placement

A session SHALL carry two distinct session-scoped layout planes: the **launch spec** (the recipe -- which
surfaces exist and the `placement` slot each occupies), set by `ApplyLaunchSpec` and read by
`GetLaunchSpec`; and the **panel-tree geometry** (how those placements are arranged into splits/tabs), set
by `ArrangePanels` and read by `GetPanelTree`. A surface SHALL bind to a session slot by its `placement`,
minted at `SpawnSurface`, resolved by `FindSurfaceByPlacement`, and released at `CloseSurface`. The two
planes SHALL be independent: changing one SHALL NOT rewrite the other. Window placement (popping a surface
into its own OS window) SHALL NOT be a domain operation (it is frontend-local chrome).

#### Scenario: Spawn mints a placement that find resolves

- **WHEN** `SpawnSurface` runs for a session
- **THEN** a placement unique within that session is minted, `FindSurfaceByPlacement(session, placement)`
  resolves to the new surface, and it resolves to none for an unused placement

#### Scenario: Recipe and geometry are independent planes

- **WHEN** `ApplyLaunchSpec` replaces the recipe and `ArrangePanels` sets the geometry
- **THEN** `GetLaunchSpec` and `GetPanelTree` each return their own plane, and setting one does not alter
  the other

