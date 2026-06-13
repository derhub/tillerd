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
working app shipping at **0.0.14**.

### 0.0.1 — Orchestrator boots, services run

A runtime-agnostic orchestrator crate the desktop embeds, with persistence and
supervised services. Nothing renders yet.

- [x] Orchestrator crate — runtime-agnostic Rust library (ADR-0022), embedded in-process
  by the desktop host; transport-agnostic API + `EventSink` bound to Tauri.
- [x] Supervised startup — orchestrator adopt-or-spawns gate + daemon; per-service health.
- [x] Persistence — `tillerd.db` (rusqlite) with the schema and lazy migration runner
  (ADR-0023).
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

- [x] Projects — create `blank` / `local-dir` / `git-repo` / `git-worktree`; name
  inference + custom + rename; list / open; Unfiled seeded.
- [x] Sessions — container CRUD; title inference (agent title | branch | both) + custom;
  add / remove surfaces; resume after restart.
- [x] Layout persistence — panel tree (`layout_json`) saved per session; restored on resume.
- [x] Archive — `deleted_at` soft-delete (cascades to surfaces); hard-delete; worktree
  directory kept.

### 0.0.5 — Launch system

Declarative startup: a session is an instance of a project's launch template. Shipped as
the **launch-execution** change (PR #12, terminal-only — ADR-0026/0027).

- [x] Launch spec — versioned JSON blob; lazy migration (ADR-0021).
- [x] Command library — prebuilt (login shell) + user-added. (The agent-CLI seed is dropped
  with the agent surface; it returns in 1.0.0.)
- [x] Launch items — target (terminal), placement (named regions), command / args / env,
  worktree step (create -> returns cwd, sets `worktree_id`). No pre/post/auto-spawn scripts:
  an auxiliary runner (e.g. a dev server) is an ordinary terminal item with a placement;
  closing the pane leaves the process running (soft-delete keeps the PTY).
- [x] Templates -> instances — a project template instantiates a session's surfaces; the
  session may diverge. (Spec-copy on session create, executor wiring, workspace IPC
  handlers, and idempotent seed all done.)
- [x] Worktrees — owned by a project; created by the worktree step.

### 0.0.6 — Finalize the architecture

The last architecture-changing version of 0.x. Everything frozen here — service
contract, wire protocol, data model (ADR-0023), extension seams, runtime layout
(ADR-0025), the panel-surface binding (ADR-0030), design tokens — holds for the rest
of 0.x; every later version is additive on these seams, never a change to them.

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
- Sidebar expand state — lands with the 0.0.14 project-tree expand/collapse UI; persists via
  this store.
- Env secrets via the OS keychain (`secret_ref` stores handles only) — no keychain dependency
  in 0.0.x.

### 0.0.10 — Notification center

User-facing event notifications: in-app persistent history and native OS banners.
Distinct from the dev log viewer (raw structured logs); this is user-facing signal.

- [ ] In-app notification center — bell icon in the app chrome (toolbar); click opens
  a popover / drawer listing recent events with timestamp and session context.
- [ ] Event types surfaced: surface started / stopped, session error, service health
  change (gate / daemon up / down), and any other user-relevant orchestrator event.
- [ ] Native OS notifications (Tauri notification plugin) — send a system banner
  (macOS / Linux) for background events when the app is not in focus. User can
  dismiss or click through to the relevant session.
- [ ] Notification history stored per app launch (in-memory or SQLite); cleared on
  quit (no cross-session persistence required in 0.0.x).
- [ ] Unread badge on the bell icon; clears on open.
- [ ] Notification center is the sole user-facing feedback channel — no Sonner toasts.

---

### 0.0.11 — Panel detach / multi-window

Tear-off panels and project windows (picture-in-picture model). Orchestrator event
sink already supports multiple concurrent subscribers (one per window); no backend
connectivity changes needed.

- [ ] Panel detach — panel header "detach" button tears the panel into a new child
  window. Parent shows a greyed-out placeholder with a "Focus →" button to bring the
  child window to front.
- [ ] Project in new window — right-click a project in the sidebar → "Open in new
  window"; parent sidebar entry shows a pending-detach indicator; clicking it focuses
  the child window.
- [ ] Re-attach — child window has a "Re-attach" action that returns the panel /
  project to its parent window and auto-focuses the parent.
- [ ] Closing the parent window does not affect detached child windows.
- [ ] Only panels with a live surface support detach; empty panels do not.
- [ ] E2E: panel detach → "Focus →" → re-attach on macOS and Linux CI.

---

### 0.0.12 — Workspace management

Project and session CRUD UX, plus session ordering. Backend (rename / delete) landed
in 0.0.4; this milestone ships the interaction design for those operations and adds
sort order.

- [ ] Inline rename — double-click a project or session row in the sidebar to rename
  in place; Enter confirms, Escape cancels.
- [ ] Delete — right-click context menu (or hover button) → "Delete"; shadcn
  AlertDialog confirmation; hard-delete cascades to surfaces (PTYs terminated).
  Distinct from archive (soft-delete, PTY preserved).
- [ ] Session reorder — drag sessions within a project to reorder; `sort_order` column
  added to `sessions` table (migration + orchestrator API); order persists across
  restarts.
- [ ] Project reorder — drag projects in the sidebar; `sort_order` on `projects` table
  (same migration pass).
- [ ] Context menus — right-click on project and session rows surfaces the full action
  list (rename, archive, delete, open in new window).
- [ ] E2E: rename, delete, and reorder flows on macOS and Linux CI.

---

### 0.0.14 — UX/UI (ships the working app)

Depends on 0.0.8 (error recovery UX), 0.0.9 (settings, preference storage),
0.0.10 (notification center), 0.0.11 (panel detach), 0.0.12 (workspace management),
0.0.13 (command center).
Exit criterion: all bullets checked + E2E suite green on macOS and Linux CI.

**Interaction polish**
- [ ] Sidebar project tree — projects expand / collapse (state persisted via 0.0.9
  window state); sessions nested under each project; hover-reveal icon buttons land in
  0.0.12; this milestone verifies cohesion and fixes any remaining interaction gaps.
- [ ] Zero state — when no projects exist, sidebar is empty and the center pane shows a
  "Create project" call-to-action.
- [ ] Empty panel picker — `EmptyPanel` lists available surface kinds; terminal is the
  only kind in 0.x.
- [ ] Pane error / failure states — surface-level error overlay distinct from the
  host-status badge (owned by 0.0.8).

**Visual polish**
- [ ] Icons, spacing, density, typography — final pass across all chrome elements.
- [ ] Panel header toolbar — split-horizontal and split-vertical icon buttons; close
  button; shadcn Tooltip on every icon-only button.
- [ ] Panel title — session name + surface kind + elapsed time since PTY spawn
  (`spawned_at` exposed by orchestrator surface state).
- [ ] Status badges — starting / running / failed on session rows.
- [ ] Terminal font and color scheme — ship a good default monospace font; color scheme
  is user-selectable (lives in 0.0.9 global settings; `DESIGN.md` terminal-* tokens
  updated from hardcoded GitHub-dark to the active scheme's mapping).
- [ ] Popups / menus — all dropdowns and dialogs use shadcn primitives and follow
  design tokens.

**Surface manipulation**
- [ ] Panel split initiation — split-H / split-V toolbar buttons create an empty leaf;
  the `EmptyPanel` picker in that leaf spawns the surface (ADR-0030 geometry model).
- [ ] Panel drag-and-drop — drag a panel leaf to swap placements with another leaf
  (new orchestrator `swap_placement` API).
- [ ] Panel resizing — drag divider to resize; double-click divider to reset to equal
  split.
- [ ] Terminal copy / paste — verified on all platforms (xterm.js default behavior;
  confirm no Tauri webview conflicts).
- [ ] Close surface — shadcn confirmation popup with "Don't ask again" checkbox;
  preference stored via 0.0.9 settings. Hard remove: drops spec item + terminates PTY
  (ADR-0030).
- [ ] Keyboard shortcuts (minimal) — native Tauri menu accelerators for: new project,
  new session, new terminal surface, close surface, switch session. Accelerators fire
  even when the terminal has keyboard focus. Full configurable shortcuts (command center landed in 0.0.13).

**Motion**
- [ ] Surface lifecycle animations — fade only: opacity 0→1 on create, 1→0 on destroy,
  using the existing `--motion-fast` / `ease-standard` tokens. No layout shift.
- [ ] Layout change animations — add / remove panels fades at the same token cadence.

**Light-mode coverage**
- [ ] Component-level appearance verified in light mode (tokens landed in 0.0.6;
  terminal canvas stays dark in both themes by design).

**Accessibility**
- [ ] ARIA labels and roles on all interactive chrome elements (sidebar, panel headers,
  dialogs, buttons, tooltips).
- [ ] Keyboard navigation in chrome — Tab / Enter / Escape through sidebar, panel
  actions, and dialogs. Terminal canvas is explicitly exempt from screen-reader support.
- [ ] Color contrast — all token pairs pass WCAG AA.

**Cross-platform polish**
- [ ] macOS — native Tauri window decorations (traffic lights); sidebar top area is a
  `data-tauri-drag-region`.
- [ ] Linux — system title bar respected; no custom decorations override.
- [ ] Platform-specific keyboard accelerator labels in menus (⌘ vs Ctrl).

**Performance**
- [ ] Sustained 60fps under multiple sessions and surfaces; profiling and optimization
  as needed.
- [ ] Low memory footprint — no unbounded growth across session switches.

**E2E coverage**
- [ ] Panel split + spawn terminal in the new leaf.
- [ ] Close surface — confirmation dialog + "Don't ask again" preference persists.
- [ ] Panel leaf drag-and-drop rearrangement.
- [ ] All flows green on macOS and Linux CI.

**Final coherence pass**
- [ ] UX/UI review cycle — dog-food, identify pain points, file follow-up issues for
  0.1.x. Any blocker found becomes a bullet here before shipping.

---

### 0.0.13 — Command center

Configurable keyboard shortcuts with a leader-key–activated command palette.

- [ ] Leader key — a configurable key sequence (e.g. Shift+Shift or Cmd+Shift)
  registered at the Tauri native-menu level so it fires even when the terminal has
  focus. Activating it opens the command center overlay.
- [ ] Command center overlay — fuzzy-searchable palette of all available actions
  (create project / session / surface, close, switch session, split panel, detach,
  …). Actions invoke the same handlers as toolbar buttons and menu accelerators.
- [ ] Configurable bindings — every action has a rebindable key; bindings stored in
  global settings (0.0.9).
- [ ] Preset profiles — ship built-in keybinding presets: `default`, `vim`, `vscode`,
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
