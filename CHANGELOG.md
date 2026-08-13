# Changelog

All notable changes to this project are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project is pre-1.0
and APIs may break between minor versions.

## [Unreleased]

Next: **agent-view-runtime** (proposal) — a sandboxed renderer for agent-authored views, a
local diff/review pipeline, and first-party MCP tools so the agent in the terminal can drive
the app's own UI. The working app shipped at **0.0.20**; the architecture froze at 0.0.6 and
every later 0.x version is additive on those seams. 0.1.x extends the working app; 1.0.0 ships
distribution and returns the agent surface (ADR-0027).

### Added

- **E2E** — session-row status badge driven by the surface-status push channel (idle → running on
  spawn); sidebar project expand/collapse persistence across a genuine app restart.
- **Panel geometry** — nested divider proportions persist per session across reloads; incompatible
  development layouts now surface a blocking error instead of silently rebuilding panel bindings.

## [0.0.20] — 2026-07-11

UX/UI — ships the working app.

### Added

- **Interaction polish** — sidebar project tree (expand / collapse, persisted); zero state
  with a "Create project" CTA; empty-panel surface picker; surface-level error overlays
  distinct from the host-status badge.
- **Visual polish** — final icon / spacing / density / typography pass; panel header toolbar
  (split-horizontal / split-vertical / close, tooltip on every icon-only button); panel title
  (session + surface kind + elapsed since spawn); session-row status badges; shadcn primitives
  across all dropdowns and dialogs.
- **Surface manipulation** — split initiation into an empty leaf; panel drag-and-drop swap
  (`swap_placement`); divider resize + double-click reset; terminal copy / paste; close-surface
  confirmation with a "don't ask again" preference; minimal native menu accelerators (new
  project / session / surface, close, switch session) that fire even with terminal focus.
- **Motion** — fade-only surface lifecycle (opacity 0↔1) and layout-change animations on the
  existing motion tokens; no layout shift.
- **Accessibility** — partial ARIA labels / roles and panel keyboard handling landed;
  complete nested-sidebar tooltip, keyboard-route, and rendered-state contrast audits remain open.
- **Cross-platform** — macOS native decorations + drag region; Linux system title bar; ⌘ vs
  Ctrl accelerator labels.
- **E2E** — panel split + spawn, close-surface confirm + preference persistence, panel-leaf
  drag-and-drop.

### Deferred

- **Performance** — sustained 60fps under multiple sessions and low-memory-footprint checks
  moved to 0.1.7: no profiling harness exists yet, so these were never measurable.

## [0.0.17] — 2026-07-03

Foundation integration.

### Added

- **End-to-end foundation** — storage + state model + TanStack proven together in one
  consolidated journey (create → switch → reload with surface identity preserved across both
  the session switch and a full window reload; multi-window coherence via the parent-row
  reaction). No integration blocker surfaced.

## [0.0.16] — 2026-06-25

Client engine — TanStack (ADR-0039, ADR-0040).

### Added

- **TanStack Router** — replaces react-router routing; file-based routes; typed search-params
  carry the `?w=<id>` window intent.
- **TanStack Query** — server-state cache is the sync axis (pending / error / stale / refetch);
  `MutationCache.onSuccess` auto-invalidates via `meta.invalidates`; no imperative `refresh()`
  remains.
- **TanStack Store** — reactive client-UI-state store; server data stays in the Query cache.
- **Multi-window coherence** — a write broadcasts its `meta.invalidates` keys over the Tauri
  event bus so other windows invalidate the matching Query key (ADR-0039).
- **State model wired through the client engine** (ADR-0044, supersedes ADR-0034) — Rust entity
  states / transitions / guards exported as a committed contract fixture with dual drift tests;
  UI action enablement derives from the mirrored guard table.
- **Workspace-activity read-model** — per-workspace running / failed rollup, kept live in every
  window by a `surface_status_changed` push; minimal activity dot on workspace rows.
- **View pointers in the settings store** — active workspace, per-project last session, and
  sidebar expansion persist as orchestrator settings keys (no webview `localStorage`); a stale
  pointer resolves to Default.
- **Server-side command-error notifications** — a failed user command is recorded by the
  orchestrator (durable, pushed to all windows); the renderer no longer records notifications.

### Removed

- **react-router framework-mode toolchain** — swapped for a Vite SPA build.

## [0.0.15] — 2026-06-22

Storage de-abstraction — sqlx (ADR-0036, supersedes ADR-0035).

### Added

- **Four-layer orchestrator** — `entities/` (pure domain, no I/O), `infra/` (per-entity async
  `sqlx` repositories + the surface runtime behind a `Runtime { Daemon | Fake }` enum),
  `shared/` (fs, kv, pagination, datetime, errors, CQS `Command`/`Query`/`Bus`), `app/` (CQS
  operations in ubiquitous language), `boot/` (composition root).
- **Runtime-bound sqlx 0.9** — one repo per entity owns its table and `Row → Entity` mapping;
  `.bind` queries with no `.sqlx`/`DATABASE_URL` build dependency (compile-time `query!` macros
  banned by ast-grep); not an ORM.
- **CQS bus** — per-command transaction boundary (`Ctx::transaction`), not the bus.
- **Caller-assigned create ids** — creates mint the id at the caller (`transport_create!`),
  removing the snapshot-then-list-diff read-back; creates are idempotent.
- **Thin transport** — `transport_command!` / `transport_query!` / `transport_create!` generate
  the `#[tauri::command]` shims; `collect_transport!` registers ~116 app-layer commands (no
  `inventory` crate); wire protocol and dynamic ACL unchanged.
- **Synchronous zero-copy event dispatch** — pull source + app pump; no per-event clone on the
  hot path.
- **Layer-boundary enforcement** — ast-grep rules gate the entities / infra / app / shared
  import boundaries; `ast-grep scan` + tests gated in CI.

### Removed

- **`rusqlite`, the `Backend` enum, `store/` wrappers, the `infra/memory` double, and the
  slug-tree machinery.** Clean cutover — domain on-disk format breaks; no migration (pre-v1).
- **Worktree** — no `git worktree add` step, no `git_worktree` source_kind, no `worktree`
  table / entity; surface = `{ id, kind, placement, cwd }`.

## [0.0.14] — 2026-06-19

Workspaces (ADR-0032).

### Added

- **Workspace tier** above project — `workspace` table + `project.workspace_id`; additive
  migration backfills every existing project into an un-deletable Default; workspace CRUD /
  reorder + `move_project`.
- **Switcher** — single main window with a workspace switcher that re-scopes the sidebar in
  place; a workspace also opens in its own window via the existing detach machinery (`?w=<id>`).
- **E2E** — create, switch, and detach flows on macOS and Linux CI.

## [0.0.13] — 2026-06-15

Command center.

### Added

- **Leader key** — a configurable sequence registered at the Tauri native-menu level so it
  fires even when the terminal has focus; opens the command center overlay.
- **Command palette** — fuzzy-searchable list of all actions, invoking the same handlers as
  toolbar buttons and menu accelerators.
- **Configurable bindings** — every action is rebindable; bindings stored in global settings.
- **Preset profiles** — built-in `default` / `vim` / `vscode` / `tmux` baselines with
  per-binding overrides.

## [0.0.12] — 2026-06-15

Project & session management.

### Added

- **Inline rename** — double-click a project or session row (Enter confirms, Escape cancels).
- **Delete** — context menu → shadcn AlertDialog confirmation; hard-delete cascades to surfaces
  (PTYs terminated); distinct from archive.
- **Reorder** — drag sessions within a project and projects in the sidebar; `sort_order` columns
  (migration + API); order persists across restarts.
- **Context menus** — full right-click action list on project and session rows.
- **E2E** — rename, delete, and reorder flows on macOS and Linux CI.

## [0.0.11] — 2026-06-14

Panel detach / multi-window.

### Added

- **Panel detach** — a panel header "detach" button tears the panel into a child window; the
  parent shows a greyed placeholder with a "Focus →" button.
- **Project in new window** — right-click a project → "Open in new window"; the parent entry
  shows a pending-detach indicator.
- **Re-attach** — returns the panel / project to its parent window and auto-focuses it.
- **Independence** — closing the parent window does not affect detached children; only panels
  with a live surface support detach.
- **E2E** — detach → "Focus →" → re-attach on macOS and Linux CI.

## [0.0.10] — 2026-06-14

Notification center (ADR-0031).

### Added

- **In-app notification feed** — a bell with an unread badge in the bottom-right chrome cluster;
  recent events with timestamp and session context, most recent first (the feed lives in the
  bottom panel's Notifications tab).
- **Event types** — surface started / stopped / error, service health change, orchestrator
  status; derived host-side behind a host-agnostic `NotificationSource` port.
- **Native OS notifications** — system banners (Tauri notification plugin) for background events
  when the app is unfocused; dismissable, brings the app forward.
- **Durable history** — persisted in the orchestrator `notification` table (`migration_v5`),
  survives restart, bounded by prune-on-insert. Reverses the original "cleared on quit".
- **Sole feedback channel** — no toasts; the model is future-ready (severity / title / detail /
  actions).

## [0.0.9] — 2026-06-14

Settings foundation.

### Added

- **Settings store** — a host-agnostic, scoped (global / project) key→value store over the
  existing `setting` table (additive; no migration), reached through an orchestrator API,
  a web-safe SDK client, and a desktop bridge. Project scope resolves over global; includes
  generic "don't ask again" keyed-boolean storage for 0.0.10.
- **Settings panel** — a gear in the bottom-right chrome cluster opens a non-modal panel
  with theme (light / dark) and terminal color-scheme controls; renders instantly.
- **Theme** — appearance is dynamic and applied from first paint (no flash) via a paint-time
  cache, with the durable choice in the settings store.
- **Terminal color scheme** — user-selectable; applied live to terminal surfaces without
  restarting them.
- **Window state** — window size / position / maximized persisted and restored on relaunch
  via `tauri-plugin-window-state`.

### Deferred

- OS-keychain env-secrets, default command / template selection, per-project overrides, and
  sidebar expand-state restore — each needs plumbing beyond this slice (keychain dependency,
  template-list API, project-scoped UI / launch-path injection, or the 0.0.14 sidebar tree);
  all build on the settings store shipped here.

## [0.0.8] — 2026-06-14

Health and first-run UX.

### Added

- **Per-service health** — the orchestrator derives rich per-service state (version,
  liveness, version-match) from the manifest and exposes it through an additive host
  command, event, and SDK type. Read-only, manifest-derived, no health socket.
- **Health indicator** — an aggregate read-only status indicator (worst-across-services
  state) in the bottom-right cluster; click opens a non-modal panel listing orchestrator
  / gate / daemon with version, liveness, failure reason, and a logs link.
  Version-mismatch and draining shown inline.
- **Progressive boot** — the shell renders immediately; service-dependent content
  lazy-loads behind a delayed skeleton; a service failure degrades to the health
  indicator rather than a blocking screen or setup wizard.

## [0.0.7] — 2026-06-14

Observability.

### Added

- **Log viewer** — a desktop log-viewer surface over the per-service structured logs,
  correlation-id aware, with per-service facet filtering.

## [0.0.6] — 2026-06-13

Finalize the architecture — the last architecture-changing version of 0.x. The service
contract, wire protocol, data model, extension seams, and design tokens freeze here;
every later version is additive on these seams.

### Added

- **Desktop E2E suite** — WebdriverIO over `tauri-webdriver`; boot / project / session /
  terminal-stream specs in dev and bundled modes, running in CI.
- **Service contract** — first-class ready / drain lifecycle phases and the socket /
  manifest discovery convention on the `Service` trait; gate and daemon conform.
- **Drain-and-restart upgrade** — replaces fd-handoff (ADR-0011): on a version mismatch
  the daemon drains, swaps the binary, and starts fresh.
- **Placement and multi-surface** — placement becomes a unique slot id so a session holds
  N surfaces; panels bind surfaces by placement; revisit resumes each by
  `(session, placement)` (ADR-0030).
- **Correlation IDs** — `correlation_id` threaded across hops in structured logs.
- **Design tokens** — `DESIGN.md` tokens applied across the shell; motion / icon-size /
  light-mode gaps closed.
- **Dynamic-ACL contract test.**

### Removed

- **Retired TS packages** — dead-code sweep deletes the Rust-inversion leftovers
  (`engine`, `platform-bun`, `adapter-claude-code`, TS `daemon-pty` / `gate-client`).

## [0.0.5] — 2026-06-13

Launch system: a session is an instance of a project's launch template. Terminal-only.

### Added

- **Launch spec** — versioned JSON blob with lazy migration; launch items carry target,
  placement, command / args / env, and a worktree step.
- **Command library** — prebuilt login shell plus user-added commands.
- **Templates to instances** — a project template instantiates a session's surfaces; the
  session may diverge.

### Removed

- **Agent surface** — 0.x is terminal-only; the agent surface (added in 0.0.3) is deferred
  to 1.0.0 (ADR-0027). The gate hook fan-out stays as shared infrastructure.

## [0.0.4] — 2026-06-12

Projects group sessions; sessions group surfaces; both persist across restart.

### Added

- **Projects** — blank / local-dir / git-repo / git-worktree; name inference + rename;
  Unfiled seeded.
- **Sessions** — container CRUD; title inference; add / remove surfaces; resume.
- **Layout persistence** — panel tree saved per session, restored on resume.
- **Archive** — `deleted_at` soft-delete cascading to surfaces; hard-delete.

## [0.0.3] — 2026-06-12

### Added

- **Agent surface and hook path** — Rust adapter, hook to status/content routing,
  status-badge UI, idempotent hook setup. (Removed in 0.0.5; returns in 1.0.0.)

## [0.0.2] — 2026-06-12

A terminal session streams through the Rust stack.

### Added

- **Surface runtime in Rust** — per-PTY proxy, status, send-queue over
  `daemon-pty-client`.
- **Terminal surface** — create a session with a terminal surface; xterm streams via the
  orchestrator API and `EventSink`.
- **Persist + resume** — a `surface` row; reconnect to the daemon by `surface_id`.

### Removed

- **TS engine** — retired for the terminal path; the surface runtime moves into the Rust
  orchestrator (ADR-0022).

## [0.0.1] — 2026-06-12

### Added

- **Orchestrator crate** — runtime-agnostic Rust library (ADR-0022), embedded in-process
  by the desktop host; transport-agnostic API + `EventSink`.
- **Supervised startup** — adopt-or-spawn gate + daemon; per-service health.
- **Persistence** — `tillerd.db` (rusqlite) with schema and lazy migration (ADR-0023).
- **SDK as API client** — the TS SDK talks to the orchestrator API.

## [0.0.0] — Baseline

Scaffolded components, pre Rust inversion.

### Added

- **PTY daemon** — detached, multi-session terminal owner with binary-framed IPC,
  session persistence, and crash recovery.
- **Gate** — single multiplexed socket fronting all agent-facing traffic; routes by
  preamble (hook, tool, subscribe, admin, mcp) with auth, normalization, and fan-out, and
  is the observability chokepoint.
- **Desktop shell** — Tauri app that orchestrates sessions (mint, register,
  adopt-or-spawn).
- **Shared libraries** — `contracts` (wire types + frame codec), `service-host`,
  `process-launch`, `gate-client`, `redact`.
- **Observability** — correlation-bound log context and resource identity (OTel-ready).

[Unreleased]: https://github.com/derhub/tillerd
