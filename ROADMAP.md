# Roadmap

Status legend: `- [ ]` planned · `- [x]` done · `[WIP]` in progress · `[HELP]` wants
input. Within a milestone, items are ordered most to least foundational. Across
milestones, cross-cutting foundations land before their consumers — a later version
must never force rework of an earlier one. Every version is a small, demoable step;
nothing is shortcut to a `.0` bucket.

> Status: 0.0.x, pre-release. The **0.0.x** line ships a **working app**: the
> foundation is in (Rust orchestrator, persistence, surface runtime, launch system —
> 0.0.1–0.0.5). **0.0.6 finalizes the architecture** — service contract, daemon
> upgrade path, correlation, design tokens, E2E test; after it, every 0.x version is
> additive on frozen seams, never a change to them. Then observability, health /
> first-run UX, settings + secrets, notifications, panel detach, workspace management,
> and a UX/UI pass complete the working app. **0.x is terminal-only**: the agent surface
> (built in 0.0.3) was removed in the launch-execution cut and is deferred to **1.0.0**
> ([ADR-0027](./docs/adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)).
> After the working app, **0.1.x extends** — diff surface, placement geometry, workflow
> library, container backend, web remote — ordered cheapest-first on the frozen seams;
> **1.0.0** is the stable horizon and ships distribution. See ADRs
> [0020](./docs/adr/0020-session-is-a-per-context-term-and-desktop-groups-surfaces.md)–[0027](./docs/adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)
> for the workspace model and the 0.0.x build. See [CHANGELOG](./CHANGELOG.md).

---

## 0.0.x — Working app

The Rust inversion, then everything a daily-usable app needs. The line ends with the
working app shipping at **0.0.20**.

### 0.0.1 — Orchestrator boots, services run

A runtime-agnostic orchestrator crate the desktop embeds, with persistence and
supervised services. Nothing renders yet.

- [x] Orchestrator crate — runtime-agnostic Rust library (ADR-0022), embedded in-process
  by the desktop host; transport-agnostic API + `EventSink` bound to Tauri.
- [x] Supervised startup — orchestrator adopt-or-spawns gate + daemon; per-service health.
- [x] Persistence — `tillerd.db` with the schema and lazy migration runner (ADR-0023).
  (Storage de-abstracted to `sqlx` per-entity repos in 0.0.15 — ADR-0036 supersedes ADR-0035;
  `rusqlite` dropped.)
- [x] SDK as API client — the TS `sdk` talks to the orchestrator API; the UI reaches
  `ready` through it (old TS engine path off). Blank UI acceptable.

### 0.0.2 — Terminal surface, end-to-end

A terminal session streams through the new Rust stack; the TS engine is retired for it.

- [x] Surface runtime in Rust — per-PTY proxy, status, send-queue over
  `daemon-pty-client`; TS engine retired.
- [x] Terminal surface — create a session (Unfiled project) with a terminal surface;
  xterm streams via the API / `EventSink`.
- [x] Persist + resume — a `surface` row; reconnect to the daemon by `surface_id` after
  restart.

### 0.0.3 — Agent surface and hook path `[reverted]`

Built and merged (#8), then **removed** in the launch-execution cut: 0.x is terminal-only
and the agent surface is deferred to **1.0.0** (ADR-0027). The gate hook fan-out stays as
shared infrastructure (mcp-gateway, memory capture). The agent surface — Rust adapter,
hook -> status/content routing, status-badge UI, idempotent hook setup — returns in 1.0.0,
with its launch command sourced from the command library.

### 0.0.4 — Projects and sessions (the container)

Projects group sessions; sessions group surfaces; both persist and survive restart.

- [x] Projects — create `blank` / `local-dir` / `git-repo`; name inference + custom + rename;
  list / open; Unfiled seeded. (`git-worktree` source kind dropped in 0.0.15.)
- [x] Sessions — container CRUD; title inference (agent title | branch | both) + custom;
  add / remove surfaces; resume after restart.
- [x] Layout persistence — panel tree (`layout_json`) saved per session; restored on resume.
- [x] Archive — `deleted_at` soft-delete (cascades to surfaces); hard-delete. (The
  worktree-directory-kept clause is moot since 0.0.15 dropped worktrees.)

### 0.0.5 — Launch system

Declarative startup: a session is an instance of a project's launch template. Shipped as
the **launch-execution** change (PR #12, terminal-only — ADR-0026/0027).

- [x] Launch spec — versioned JSON blob; lazy migration (ADR-0021).
- [x] Command library — prebuilt (login shell) + user-added. (The agent-CLI seed is dropped
  with the agent surface; it returns in 1.0.0.)
- [x] Launch items — target (terminal), placement (named regions), command / args / env,
  worktree step (create -> returns cwd, sets `worktree_id`; the worktree step + entity were
  dropped in 0.0.15). No pre/post/auto-spawn scripts:
  an auxiliary runner (e.g. a dev server) is an ordinary terminal item with a placement;
  closing the pane leaves the process running (soft-delete keeps the PTY).
- [x] Templates -> instances — a project template instantiates a session's surfaces; the
  session may diverge. (Spec-copy on session create, executor wiring, workspace IPC
  handlers, and idempotent seed all done.)
- [x] Worktrees — owned by a project; created by the worktree step. (Removed in 0.0.15:
  worktree provisioning + entity dropped.)

### 0.0.6 — Finalize the architecture

The last architecture-changing version of 0.x. Everything frozen here — service
contract, wire protocol, data model (ADR-0023), extension seams, runtime layout
(ADR-0025), the panel-surface binding (ADR-0030), design tokens — holds for the rest
of 0.x; every later version is additive on these seams, never a change to them.
(0.0.15 later de-abstracted the storage *implementation* — sqlx per-entity repos,
ADR-0036 — beneath this frozen data model, wire protocol, and ACL: the seams held, the
internal layering changed.)

- [x] Desktop E2E suite — first, so every later milestone verifies against it instead
  of manual checks. The rig exists (`tests/desktop-e2e/run.sh`: WebdriverIO +
  `tauri-webdriver` over test-gated `tauri-plugin-webdriver`; macOS WKWebView works,
  unlike official `tauri-driver` — tauri#7068; smoke specs: boot, project / session,
  terminal stream). Extend to a solid suite: boot to ready in both dev and bundled
  modes, full create flows, resume after restart, runs in CI. (Agent render deferred
  to 1.0.0 with the agent surface.)
- [x] Dynamic-ACL contract test (deferred from the GUI arg-shape work).
- [x] Solidify `service-host`: add first-class ready / drain lifecycle phases and the
  discovery convention (socket / manifest) to the `Service` trait — health (ADR-0019)
  and identity / version are already in. Gate + daemon conform; future services
  inherit the contract. (Health feeds the 0.0.8 indicators.)
- [x] Replace fd-handoff (ADR-0011) with drain-and-restart: on a version mismatch the
  daemon drains (refuses new sessions, lets active ones finish), swaps the binary,
  starts fresh. Builds on the contract's drain primitive — re-check the
  `daemon-upgrade-drain-restart` proposal against it before implementing.
- [x] Placement and multi-surface: placement becomes a unique slot id (not named
  regions) so a session holds N surfaces; panels bind surfaces by placement; revisit
  resumes each by `(session, placement)`; spawning a surface diverges the session's
  launch spec ([ADR-0030](./docs/adr/0030-panels-bind-surfaces-by-placement.md)). The
  panel-surface seam freezes here; the terminal revisit already shipped is the first
  slice. Sizes / nested splits stay 0.1.x (additive geometry).
- [x] `correlation_id` threaded across hops in structured logs — the log-viewer
  (0.0.7), health surfacing (0.0.8), and every later feature join records on it.
- [x] Design tokens: apply [`DESIGN.md`](./apps/ui/DESIGN.md) across the existing
  shell and close its token-level gaps (motion / transition scale, icon sizing
  token, light-mode tokens) — all later UI (log-viewer, health, onboarding,
  settings) is built on final tokens.
- [x] Dead-code sweep: delete the retired TS packages left from the Rust inversion
  (`engine`, `platform-bun`, `adapter-claude-code`, TS `daemon-pty` / `gate-client`,
  ...) where nothing live references them; dormant `apps/server` keeps only what it
  needs until its 0.1.4 rewrite.

### 0.0.7 — Observability

- [x] Log-viewer surface in the desktop, over the 0.0.6 structured logs.

### 0.0.8 — Health and first-run UX

- [x] Per-service health indicators (gate / daemon) with failure surfacing — an
  aggregate read-only indicator in the bottom-right cluster (worst-across-services
  state); click opens a non-modal panel listing orchestrator / gate / daemon with
  version, liveness, failure reason, and a logs link; version-mismatch and draining
  shown inline. Manifest-derived, no health socket.
- [x] First-run / onboarding — progressive, invisible boot: the shell renders
  immediately, service-dependent content lazy-loads behind a delayed skeleton, and a
  failure (services down, version out of range) degrades to the subtle health
  indicator rather than a blocking modal or setup wizard. Recovery stays read-only
  (failure reason + logs link, no retry/restart — supervision seam frozen);
  interactive per-error recovery prompts deferred. 0.0.10 inherits this surfacing.

### 0.0.9 — Settings

The settings foundation: a host-agnostic, scoped settings store (orchestrator `setting`
table) reached through a port + SDK client + desktop bridge, with the settings panel,
theme, terminal color scheme, and window state. Secrets and the heavier consumers are
deferred (each needs plumbing this slice does not budget); they build on the store shipped
here.

- [x] Settings store — scoped (global / project) key→value over the `setting` table, host
  agnostic (orchestrator API → SDK client → desktop bridge), incl. "don't ask again"
  keyed-boolean storage (used by 0.0.10).
- [x] Global settings panel: theme (light / dark, applied from first paint), terminal color
  scheme; opened from the bottom-right chrome cluster.
- [x] Window size / position / maximized persisted and restored on relaunch
  (`tauri-plugin-window-state`).

Deferred to follow-up changes (each builds on the store shipped here):

- Default command library / default template selection — needs a template-list API + an SDK
  command-list method.
- Per-project overrides (launch template, project env) — needs a project-scoped settings UI +
  launch-executor env injection.
- Sidebar expand state — lands with the 0.0.20 project-tree expand/collapse UI; persists via
  this store.
- Env secrets via the OS keychain (`secret_ref` stores handles only) — no keychain dependency
  in 0.0.x.

### 0.0.10 — Notification center

User-facing event notifications: in-app persistent history and native OS banners.
Distinct from the dev log viewer (raw structured logs); this is user-facing signal.

- [x] In-app notification center — bell in the bottom-right chrome cluster; click opens
  a popover listing recent events with timestamp and session context, most recent first.
- [x] Event types surfaced: surface started / stopped / error, service health change
  (gate / daemon up / down), and orchestrator status. Derived host-side from existing
  signals behind a host-agnostic `NotificationSource` port (server adapter deferred).
- [x] Native OS notifications (Tauri notification plugin) — system banner (macOS / Linux)
  for background events when the app is unfocused; dismissable, brings the app forward
  (the in-app feed does the per-session click-through).
- [x] Notification history is durable — persisted in the orchestrator `notification` table
  (additive `migration_v5`), survives restart, bounded by prune-on-insert. Reverses the
  original "cleared on quit" ([ADR-0031](./docs/adr/0031-notifications-persist-in-the-orchestrator-store.md)).
- [x] Unread badge on the bell; clears on open.
- [x] Notification center is the sole user-facing feedback channel — no toasts. Model is
  future-ready (severity / title / detail / actions, open category union).

---

### 0.0.11 — Panel detach / multi-window

Tear-off panels and project windows (picture-in-picture model). Orchestrator event
sink already supports multiple concurrent subscribers (one per window); no backend
connectivity changes needed.

- [x] Panel detach — panel header "detach" button tears the panel into a new child
  window. Parent shows a greyed-out placeholder with a "Focus →" button to bring the
  child window to front.
- [x] Project in new window — right-click a project in the sidebar → "Open in new
  window"; parent sidebar entry shows a pending-detach indicator; clicking it focuses
  the child window.
- [x] Re-attach — child window has a "Re-attach" action that returns the panel /
  project to its parent window and auto-focuses the parent.
- [x] Closing the parent window does not affect detached child windows.
- [x] Only panels with a live surface support detach; empty panels do not.
- [x] E2E: panel detach → "Focus →" → re-attach on macOS and Linux CI.

---

### 0.0.12 — Project & session management

Project and session CRUD UX, plus session ordering. Backend (rename / delete) landed
in 0.0.4; this milestone ships the interaction design for those operations and adds
sort order.

- [x] Inline rename — double-click a project or session row in the sidebar to rename
  in place; Enter confirms, Escape cancels.
- [x] Delete — right-click context menu (or hover button) → "Delete"; shadcn
  AlertDialog confirmation; hard-delete cascades to surfaces (PTYs terminated).
  Distinct from archive (soft-delete, PTY preserved).
- [x] Session reorder — drag sessions within a project to reorder; `sort_order` column
  added to `sessions` table (migration + orchestrator API); order persists across
  restarts.
- [x] Project reorder — drag projects in the sidebar; `sort_order` on `projects` table
  (same migration pass).
- [x] Context menus — right-click on project and session rows surfaces the full action
  list (rename, archive, delete, open in new window).
- [x] E2E: rename, delete, and reorder flows on macOS and Linux CI.

---

### 0.0.14 — Workspaces

A `workspace` tier above `project` (ADR-0032): a named group of projects that owns its
own window. Strict containment — every project belongs to exactly one workspace — with a
single un-deletable Default workspace that existing projects migrate into. Additive schema
migration under the 0.0.6 data-model freeze.

- [x] Schema — `workspace` table + `project.workspace_id`; additive migration backfills
  every existing project into Default; orchestrator workspace CRUD/reorder + `move_project`.
- [x] Switcher — single main window with a workspace switcher; selecting a workspace
  re-scopes the sidebar to its projects in place.
- [x] Detach — a workspace opens in its own window via the existing project/session detach
  machinery (`window_open` / `window_focus`, `?w=workspace`).
- [x] Glossary + ADR — CONTEXT.md Workspace term; ADR-0032 extends ADR-0023.
- [x] E2E: create, switch, and detach flows on macOS and Linux CI.

---

### Foundation — storage + client engine (0.0.15–0.0.17)

> The substrate the working app needs before the 0.0.20 UX/UI ship. The original plan — a
> snapshot-tree file store + two-plane `state.db` + a state-model contract (ADR-0033/0034) —
> was **abandoned**: it fused persistence with domain logic and walked relational data as a
> directory tree. Replaced by **de-abstracted sqlx storage** (ADR-0036, accepted, supersedes
> ADR-0035). Ordered by dependency: storage de-abstraction (0.0.15) → client engine (0.0.16)
> → integration (0.0.17). The locked decisions below apply across all three.

**Locked decisions (ADR-0036):**
- **Domain hierarchy = `workspace → project → session → surface`.** Unchanged at the model
  level (ADR-0023); its slug-tree representation is dropped.
- **Four layers, one job each.** `entities/` are pure domain (types + rules — guards, the
  rename-sets-`title_source`-`Custom` rule, the cascade policy — no infra trait, no I/O);
  `infra/` is all infrastructure (per-entity async `sqlx` repositories + the surface runtime);
  `shared/` holds reusable primitives (`fs`, `kv`, `page`, `datetime`, `errors`, and the CQS
  `Command`/`Query` + `Bus`), not a storage abstraction; `app/` is a CQS layer of
  command/query objects on the bus; `boot/` is the composition root.
- **Domain data in sqlite via `sqlx` 0.9** (async, runtime-bound queries — `.bind`, no
  `.sqlx`/`DATABASE_URL` build dependency; the compile-time `query!` macros are banned by
  ast-grep; not an ORM).
  One repo per entity owns its table, columns, and `Row → Entity` mapping, with typed
  `create`/`get`/`list(parent, page)`/`update`/`delete` over an executor (pool or tx ref).
  Nesting is a `parent_id` column; rename/move/archive are `UPDATE`s; cascades are
  `UPDATE`/`DELETE`. `rusqlite`, the `Backend { Fs | Sqlite | Memory }` enum, the `store/`
  wrappers, the `infra/memory` double, and the slug-tree machinery are deleted.
- **Operations in ubiquitous language, not generic CRUD** — `New*`/`Rename*`/`Reorder*`/
  `MoveProject`/`Archive*`/`Discard*`/`SpawnSurface`; reads are descriptive `Get*`/`List*`.
  Each is a `Command<Ctx>` (mutate → `()`) or `Query<Ctx>` (read → `Out`); **the transaction
  boundary is per command** (`Ctx::transaction`), not the bus.
- **Transport is thin over a transport-agnostic core.** The tauri crate's
  `transport_command!`/`transport_query!` (`type => action`) generate the per-operation
  `#[tauri::command]` shim + `collect_transport!` registration (wire and dynamic ACL unchanged); the
  future web host owns its own macro (axum) over the same commands and bus.
- **User-config stays file-based** through `shared::fs` — settings, theme, keybindings, and the
  profile store (`config/profiles/<id>.json`, `active.json`). Profiles ship file-based here.
- **Clean cutover, no migration** (pre-v1; dev-only data discarded). The domain on-disk format
  breaks (slug-tree → sqlite); the IPC contract, dynamic ACL, and wire protocol are unchanged.
- **Deferred** (ADR-0036 out-of-scope, land in a later slice): Stronghold secrets vault +
  OS-keychain master password; the settings-profile cascade (workspace/project overrides,
  templates); the state-model contract (ADR-0034 — lifecycle FSM, sync status, guards). The
  snapshot-tree two-plane design (ADR-0033) is superseded, not deferred.

```
<app-data>/tillerd/        ALWAYS machine-local
  tillerd.db               domain + operational data (sqlite via sqlx); migrations in-tree
  config/                  user-config, file-based via shared::fs
    profiles/<id>.json     settings profiles; active.json names the active profile
    global.json            global settings scope
    project/<id>.json      per-project settings scope
```
Domain entities (workspace, project, session, surface, command, launch_template, notification)
are typed sqlite rows. Nesting is a `parent_id` column, not a directory tree; ordering via a
`sort_order` column; archive via `archived_at` (soft-delete). Refs use the stable `id`, and URL
window intent carries it (`?w=<id>`); `cwd` is relative to the project `rootPath`. The earlier
relocatable/syncable domain tree, per-entity baseline snapshots, and id→path index are gone with
the slug-tree; profiles and settings remain hand-editable JSON.

### 0.0.15 — Storage de-abstraction (sqlx)

De-abstract the orchestrator data layer (ADR-0036, accepted, supersedes ADR-0035): four layers
each with one job, domain data in sqlite via `sqlx`, operations as CQS objects on a bus. The
slug-tree, the `Backend` enum, the `store/` wrappers, and the `infra/memory` double are deleted.

- [x] `entities/` pure domain — types + rules (the `is_default`/`is_unfiled` guards, the
  rename-sets-`title_source`-`Custom` rule, the cascade policy); no infra trait, no I/O.
- [x] `infra/` per-entity async `sqlx` repositories — one repo per entity owning its table,
  columns, and `Row → Entity` mapping, with typed `create`/`get`/`list(parent, page)`/`update`/
  `delete` over an executor (pool or tx ref). Nesting via `parent_id`; rename/move/archive are
  `UPDATE`s; cascades `UPDATE`/`DELETE`. sqlx 0.9 runtime-bound (`.bind`, no `.sqlx` build dep;
  `query!` macros banned by ast-grep), not an ORM; `rusqlite` dropped.
- [x] Surface runtime into `infra/` — PTY proxies + daemon client behind a `Runtime` enum
  `{ Daemon | Fake }` (static dispatch); the `surface/` and `launch/` dirs are removed and their
  contents redistributed (`launch/spec.rs` → `entities/`; executor/api → `app/` surface commands).
- [x] `shared/` building blocks — `fs`, `kv` (`SqliteKv` + `MemoryKv`), `page` (`Page`/`Listing`),
  `datetime`, `errors`, and the CQS machinery (`Command<Cx>`/`Query<Cx>` + `Bus<Cx>`). No generic
  `Repository` trait.
- [x] `app/` CQS layer — operations in ubiquitous language; per-command transaction boundary
  (`Ctx::transaction`), not the bus; `Ctx` holds the pool, kv, config root, and `Runtime`; `boot/`
  opens the pool and builds `Ctx` + `Bus`.
- [x] Caller-assigned create ids — creates mint the id at the caller (`transport_create!`),
  removing the snapshot-then-list-diff read-back; creates are idempotent.
- [x] Thin transport over a transport-agnostic core — `transport_command!`/`transport_query!`/
  `transport_create!` generate the `#[tauri::command]` shims; the hand-maintained
  `collect_transport!` macro registers them into `tauri::generate_handler!` (no `inventory`
  crate); ~116 app-layer commands exposed; wire protocol and dynamic ACL unchanged.
- [x] Standardized event dispatch — synchronous zero-copy dispatch (pull source + app pump); no
  per-event clone on the hot path.
- [x] Layer-boundary enforcement — ast-grep rules gate the entities/infra/app/shared import
  boundaries (entities + infra stay app-internal); `ast-grep scan` + tests gated in CI.
- [x] Drop worktree provisioning + entity — no `git worktree add` step, no `git_worktree`
  source_kind, no `worktree` table; surface = `{ id, kind, placement, cwd }`; CONTEXT.md term removed.
- [x] Clean cutover — domain on-disk format breaks (slug-tree → sqlite); no migration (pre-v1).

Deferred to a later slice (ADR-0036 out-of-scope): Stronghold secrets vault + OS-keychain master
password; the settings-profile cascade (workspace/project overrides, templates — profiles ship
file-based here); the state-model contract (ADR-0034 — lifecycle FSM, sync status, guards). The
snapshot-tree two-plane storage (ADR-0033) is superseded.

### 0.0.16 — Client engine: TanStack

The real client engine (Feature C). Move to TanStack Router + Query + Store — for ecosystem
cohesion and typed search-params that fit the `?w=<id>` window-intent model (SPA throughout;
SSR not a factor). Swaps react-router's framework-mode toolchain (`build`/`dev`/`serve`,
`@react-router/node`) for a Vite SPA build.

> Re-scoped (resolved): the bullets below referenced the state-model contract and the
> hand-editable file tree that ADR-0036 deferred / superseded. Domain is now sqlite (not
> hand-edited files), so 3-way file merge / Re-sync is moot; view pointers, guards, and
> workspace-activity were re-based onto ADR-0044 (Rust-authoritative state model) and landed
> below. 0.0.17's Re-sync / conflict-prompt bullet is dropped for the same reason (see 0.0.17).

- [x] TanStack Router — replace react-router routing (12 files); typed search-params carry window intent.
  (`@tanstack/react-router`, file-based routing per ADR-0040; `app/router.tsx` + `app/lib/windows.ts`
  `WindowIntent` search-params; commit `4cee5cd6`.)
- [x] TanStack Query — server-state cache = the sync axis (pending/error/stale/refetch); kills imperative `refresh()`.
  (`app/lib/queryClient.ts` — `MutationCache.onSuccess` auto-invalidates via `meta.invalidates`; no
  imperative `refresh()` remains.)
- [x] TanStack Store — reactive client store; coherent lists across windows.
  (`app/lib/store.ts` — `uiStore = new Store<UiState>(...)`, client-UI-state only; server data stays
  in the Query cache.)
- [x] Internal multi-window coherence — windows invalidate the matching Query key on a write
  (app-internal, not file-watching). (Landed as a client-broadcast, not an orchestrator-pushed
  `changed{id}` event as originally worded: `app/lib/crossWindowSync.ts` broadcasts `meta.invalidates`
  keys over the Tauri event bus from the mutating window; ADR-0039 documents the deviation.)
- [x] Wire view pointers + state-model guards + workspace-activity read-model through Query/Store.
  (Landed re-based on ADR-0044, which supersedes ADR-0034: Rust-authoritative state/guard
  contract with dual drift tests; view pointers in the settings store; server-derived
  activity rollup kept live by the `surface_status_changed` push.)

### 0.0.17 — Foundation integration

Buffer + integration pass: the three slices proven together end-to-end before the 0.0.20 UX/UI ship.

- [x] End-to-end — storage + state model + TanStack working as one across create / switch / reload / multi-window.
  (Consolidated journey `tests/desktop-e2e/foundation-integration.test.ts`: one project threaded through
  create → switch → reload, asserting surface identity survives both the session switch and a full window
  reload; multi-window coherence via the parent-row reaction.)
- [x] Absorb any blocker found while splitting; anything deferred from 0.0.15–0.0.16 lands here.
  (No integration blocker surfaced — the consolidated journey and the full verify gate are green as-is.)

> Dropped: **Re-sync UX — placement + conflict-prompt (Override / Force-merge)**. Moot post
> ADR-0036 — conflict-locking and the 3-way file-merge model it prompted are gone with the
> hand-editable file tree; the domain is a single relational store where every mutation is one
> atomic transaction. ADR-0044 makes the server-state cache the sync axis (pending / error /
> stale), and ADR-0039's cross-window cache-invalidation broadcast already delivers multi-window
> coherence — a write in one window invalidates the matching query in the others. No conflict
> prompt is built.

---

### 0.0.20 — UX/UI (ships the working app)

Depends on 0.0.8 (error recovery UX), 0.0.9 (settings, preference storage),
0.0.10 (notification center), 0.0.11 (panel detach), 0.0.12 (project & session
management), 0.0.13 (command center), 0.0.14 (workspaces).
Exit criterion: all bullets checked + E2E suite green on macOS and Linux CI.

**Interaction polish**
- [x] Sidebar project tree — projects expand / collapse (state persisted via 0.0.9
  window state); sessions nested under each project; hover-reveal icon buttons land in
  0.0.12; this milestone verifies cohesion and fixes any remaining interaction gaps.
- [x] Zero state — when no projects exist, sidebar is empty and the center pane shows a
  "Create project" call-to-action.
- [x] Empty panel picker — `EmptyPanel` lists available surface kinds; terminal is the
  only kind in 0.x.
- [x] Pane error / failure states — surface-level error overlay distinct from the
  host-status badge (owned by 0.0.8).

**Visual polish**
- [x] Icons, spacing, density, typography — final pass across all chrome elements.
- [x] Panel header toolbar — split-horizontal and split-vertical icon buttons; close
  button; shadcn Tooltip on every icon-only button.
- [x] Panel title — session name + surface kind + elapsed time since PTY spawn
  (`spawned_at` exposed by orchestrator surface state).
- [x] Status badges — starting / running / failed on session rows.
- [x] Terminal font and color scheme — ship a good default monospace font; color scheme
  is user-selectable (lives in 0.0.9 global settings; `DESIGN.md` terminal-* tokens
  updated from hardcoded GitHub-dark to the active scheme's mapping).
- [x] Popups / menus — all dropdowns and dialogs use shadcn primitives and follow
  design tokens.

**Surface manipulation**
- [x] Panel split initiation — split-H / split-V toolbar buttons create an empty leaf;
  the `EmptyPanel` picker in that leaf spawns the surface (ADR-0030 geometry model).
- [x] Panel drag-and-drop — drag a panel leaf to swap placements with another leaf
  (new orchestrator `swap_placement` API).
- [x] Panel resizing — drag divider to resize; double-click divider to reset to equal
  split.
- [x] Terminal copy / paste — verified on all platforms (xterm.js default behavior;
  confirm no Tauri webview conflicts).
- [x] Close surface — shadcn confirmation popup with "Don't ask again" checkbox;
  preference stored via 0.0.9 settings. Hard remove: drops spec item + terminates PTY
  (ADR-0030).
- [x] Keyboard shortcuts (minimal) — native Tauri menu accelerators for: new project,
  new session, new terminal surface, close surface, switch session. Accelerators fire
  even when the terminal has keyboard focus. Full configurable shortcuts (command center landed in 0.0.13).

**Motion**
- [x] Surface lifecycle animations — fade only: opacity 0→1 on create, 1→0 on destroy,
  using the existing `--motion-fast` / `ease-standard` tokens. No layout shift.
- [x] Layout change animations — add / remove panels fades at the same token cadence.

**Light-mode coverage**
- [x] Component-level appearance verified in light mode (tokens landed in 0.0.6;
  terminal canvas stays dark in both themes by design).

**Accessibility**
- [x] ARIA labels and roles on all interactive chrome elements (sidebar, panel headers,
  dialogs, buttons, tooltips).
- [x] Keyboard navigation in chrome — Tab / Enter / Escape through sidebar, panel
  actions, and dialogs. Terminal canvas is explicitly exempt from screen-reader support.
- [x] Color contrast — all token pairs pass WCAG AA.

**Cross-platform polish**
- [x] macOS — native Tauri window decorations (traffic lights); sidebar top area is a
  `data-tauri-drag-region`.
- [x] Linux — system title bar respected; no custom decorations override.
- [x] Platform-specific keyboard accelerator labels in menus (⌘ vs Ctrl).

**Performance**
- [ ] Sustained 60fps under multiple sessions and surfaces; profiling and optimization
  as needed.
- [ ] Low memory footprint — no unbounded growth across session switches.

**E2E coverage**
- [x] Panel split + spawn terminal in the new leaf.
- [x] Close surface — confirmation dialog + "Don't ask again" preference persists.
- [x] Panel leaf drag-and-drop rearrangement.
- [ ] All flows green on macOS and Linux CI.

**Final coherence pass**
- [ ] UX/UI review cycle — dog-food, identify pain points, file follow-up issues for
  0.1.x. Any blocker found becomes a bullet here before shipping.

---

### 0.0.13 — Command center

> Placed here (after 0.0.20) because it shipped later than its number; it is a dependency of
> the 0.0.20 UX ship. Numbers 0.0.18 and 0.0.19 are intentionally unused — the foundation
> slices (0.0.15–0.0.17) landed straight into the 0.0.20 UX ship.

Configurable keyboard shortcuts with a leader-key–activated command palette.

- [x] Leader key — a configurable key sequence (e.g. Shift+Shift or Cmd+Shift)
  registered at the Tauri native-menu level so it fires even when the terminal has
  focus. Activating it opens the command center overlay.
- [x] Command center overlay — fuzzy-searchable palette of all available actions
  (create project / session / surface, close, switch session, split panel, detach,
  …). Actions invoke the same handlers as toolbar buttons and menu accelerators.
- [x] Configurable bindings — every action has a rebindable key; bindings stored in
  global settings (0.0.9).
- [x] Preset profiles — ship built-in keybinding presets: `default`, `vim`, `vscode`,
  `tmux`. User selects a preset as a baseline and can override individual bindings.

---

## 0.1.x — Enhancement and extension

Prove the seams and scale the surface area. Ordered cheapest-first; every step is
additive on the architecture frozen at 0.0.6.

### 0.1.0 — Diff surface

- [ ] Wire the diff panel as a surface kind — the second surface-kind implementation,
  pressure-testing the extension seam on the placement model (frozen in 0.0.6). (Agent
  adapters are validated in 1.0.0, after the agent surface returns — ADR-0027.)

### 0.1.1 — Placement geometry

- [ ] Sizes and nested splits on top of the 0.0.6 placement model (additive geometry).

### 0.1.2 — Prebuilt workflow library

- [ ] Bespoke workflow sessions and dev-setup presets.

### 0.1.3 — Container execution backend

- [ ] Dev-container spec / OCI runtimes behind the launch-item contract (execution-backend
  seam).

### 0.1.4 — Web remote control

- [ ] Revive the server as a Rust host embedding the same orchestrator.
- [ ] SDK over HTTP / WS; auth for remote access.

### 0.1.5 — Docs reconciliation

- [ ] README and guides match the shipped architecture.

### 0.1.6 — Terminal experience follow-ups

- [ ] Shell integration — prompt marking (OSC 133 or equivalent) so the terminal knows
  prompt / command / output boundaries: scroll-to-prompt, per-command duration and
  exit-status decoration.
- [ ] cwd inheritance — a new terminal surface spawns in the previously-focused
  surface's current directory instead of the project root.
- [ ] Session scrollback restore — a session's terminal scrollback persists and
  rehydrates on resume / reload, beyond the live PTY reconnect.
- [ ] Auto-update — the desktop app checks for and installs new releases in place.
- [ ] PTY output flow control — the renderer's write backlog is unbounded when a
  producer outruns the parser (measured ~100MB/s of retained heap under a full-speed
  generator); pause/resume the PTY on backlog high-water marks.

---

## Parked

On the line, not scheduled:

- **memorya** (`apps/memorya`) — knowledge capture over the gate's hook fan-out;
  dormant until the agent surface returns (1.0.0+).
- **mcp-gateway** (`apps/mcp-gateway`) — agent-facing MCP front (ADR-0013–0015);
  dormant and unsupervised in 0.x; returns with the agent surface (1.0.0).
- **`apps/server`** — dormant; revived in Rust as the web host at 0.1.4.
- **CLI** (`apps/cli`) — thin daemon-status tool; stays minimal until a real
  controller need appears.

---

## 1.0.0 — Stable horizon

- [ ] Stable, versioned API and launch-spec schema.
- [ ] Extension contract (surface kinds, command library, execution backends) proven by
  real second implementations.
- [ ] Cross-platform desktop with a polished, stable UX and solid performance.
- [ ] Agent as a first-class surface kind, with a rich status model and content stream over the
  gate's hook fan-out (deferred from 0.x — ADR-0027).
- [ ] Distribution: signed, notarized bundles (dmg / AppImage / deb) across macOS +
  Linux `[HELP]` (Windows?); auto-update; release pipeline — versioned releases +
  generated changelogs via changesets.
