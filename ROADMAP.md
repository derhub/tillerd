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

### Foundation — shared design (0.0.15–0.0.18)

> The substrate the working app needs before the 0.0.20 UX/UI ship: a finalized storage
> model, a standard state model, and the real client engine. Design is **solidified**
> (decisions locked below); split into four versions, each extracted into its own OpenSpec
> change. Ordered by dependency: storage (0.0.15) → state model (0.0.16) → client engine
> (0.0.17) → integration (0.0.18). The shared invariants + folder structure below apply
> across all four.

**Shared design invariants (locked):**
- **Domain hierarchy = `workspace → project → session`.** Profile is NOT a tier — it is a
  portable settings bundle (below). Domain (workspace/project/session/panel layout + surface
  bindings) = readable JSON snapshot tree. Operational (runtime status, notifications, command
  lib, worktrees, id→path index, view pointers, baseline hashes) = `state.db` SQLite,
  gitignored, regenerable. An entity lives in ONE plane, never both.
- **Profile = portable, named settings bundle** (the VS Code model). Owns settings only — no
  domain. Switchable, shareable. **Settings cascade:** `effective(project) = merge(active
  profile, workspace override?, project override?)` — workspace and project may each carry an
  optional `settings.jsonc`; session inherits, no override. Switching profile only re-resolves
  effective settings (hot-apply / reload-notify) — it does NOT touch domain or PTYs.
- **Storage-agnostic.** The app only requires the domain tree be readable at its path; how the
  user stores/versions it (git, sync, backup) is their business. Same for portable profiles.
- **Zero file watchers.** All files reconcile at **startup + an explicit Re-sync button** —
  domain and settings alike.
- **Humans may edit any file.** Lazy conflict detection: at write + Re-sync, compare file hash
  vs per-entity baseline → 3-way field merge. Disjoint fields auto-merge (inferred); overlap /
  structural change → **prompt: Override (ours wins) / Force-merge (file as base, replay ours
  on top)**. Uniform across settings + domain. No event sourcing, no continuous watching, no
  conflict markers.
- **Clean cutover, no migration** (pre-v1; dev-only data discarded).

```
<app-config>/tillerd/config.jsonc          machine-local: activeProfile, paths, app prefs
<profiles>/<profile-name>/
  settings.jsonc                            portable settings bundle (switchable, shareable)
<data-root>/
  state.db                                  operational, gitignored, regenerable
  workspaces/<ws-slug>/   workspace.json    { id, name, sortOrder }   (slug dir, stable id)
    settings.jsonc                          OPTIONAL workspace settings override
    projects/<proj-slug>/ project.json      { id, name, sourceKind, rootPath, sortOrder }
      settings.jsonc                        OPTIONAL project settings override
      .archive/<…>/                         archived subtrees (atomic move)
      sessions/<sess-slug>/ session.json    { id, title, titleSource, spec, sortOrder }
        layout.json                         panel tree (geometry) + surface bindings
                                            surface = { id, kind, placement, cwd, worktreeId }
```
Containment encodes hierarchy (no `workspace_id`/`project_id` fields); refs use stable `id`;
ordering via explicit `sortOrder`; archive = move subtree to `.archive/`.

### 0.0.15 — Storage & settings profiles

The storage substrate (Feature A). Needs an ADR (new pattern; moves the 0.0.6 storage seam).

- [ ] ADR — two-plane storage (snapshot tree + operational `state.db`) + settings-profile cascade.
- [ ] Snapshot tree store — `workspace → project → session`; slug dirs + stable `id`, containment hierarchy, `sortOrder`, atomic write-temp-rename, `.archive/` subtree move; replaces SQLite domain tables.
- [ ] `state.db` operational store — runtime status, notifications, command lib, worktrees, id→path index, view pointers, per-entity baseline hashes.
- [ ] Settings profiles — portable named bundle (`<profiles>/<name>/settings.jsonc`); `config.jsonc` holds active-profile pointer + paths; switch = re-resolve effective settings only (no domain teardown); CONTEXT.md term.
- [ ] Settings cascade — `merge(active profile, workspace override?, project override?)`; optional `settings.jsonc` at workspace + project; hot-apply where safe, else reload-notify.
- [ ] Reconcile — startup scan + Re-sync command; lazy 3-way merge (auto-merge disjoint, prompt Override/Force-merge on overlap); no watchers.
- [ ] Surface reattach on reload — diff `layout.json` placements → `detach` removed / `resume` added.

### 0.0.16 — State model

Standardize the state model (Feature B) over the 0.0.15 storage. Needs an ADR (new pattern: state-model-as-contract).

- [ ] ADR — state-model-as-contract (lifecycle / sync / guards; authority split).
- [ ] `contracts/state-model.json` (+ `.schema.json`) — single source; loaded both sides (Rust `include_str!`+serde, TS import+zod), no codegen.
- [ ] Lifecycle FSM — shared CRUD (Creating→Active→Archiving→Archived→Deleting); surface special (Spawning→Attaching→Live→Closing→Closed). Contract marks persistable vs runtime-only states.
- [ ] Surface status split — runtime `ProxyState` (Spawning/Attaching/Closing, in-memory, rebuilt at boot via `resume_all`) vs persisted typed `last_status` (Live | Exited | Crashed, `state.db`) gating resume-on-boot; replaces the free-form string.
- [ ] Sync status — `Confirmed | Pending | Rejected | Stale | Conflicted`; optimistic with rollback; `Conflicted` locks entity until resolved.
- [ ] Guards — `*-ing` states locked; only stable states accept actions; orchestrator enforces, client advisory.
- [ ] View pointers — global in `state.db`: `activeWorkspace`, `sidebar.expanded.<proj>`, `lastSession.<proj>`; resolved against live lifecycle (Active only, else default). Per-window context comes from URL intent; `focusedLeaf` stays in-memory (not persisted).
- [ ] Contract test — UI and server guards agree (like `command_contract.rs`).

### 0.0.17 — Client engine: TanStack

The real client engine (Feature C). Drop react-router's data model (routes render `null`, no
loaders — paying for an unused router). Move to TanStack Router + Query + Store.

- [ ] TanStack Router — replace react-router routing.
- [ ] TanStack Query — server-state cache = the sync axis (pending/error/stale/refetch); kills imperative `refresh()`.
- [ ] TanStack Store — reactive client store; coherent lists across windows.
- [ ] Internal multi-window coherence — orchestrator emits `changed{id}` on its own writes → windows invalidate the matching Query key (app-internal, not file-watching).
- [ ] Wire view pointers + state-model guards through Query/Store.

### 0.0.18 — Foundation integration

Buffer + integration pass: the three slices proven together end-to-end before the 0.0.20 UX/UI ship.

- [ ] End-to-end — storage + state model + TanStack working as one across create / switch / reload / Re-sync / multi-window.
- [ ] Re-sync UX — placement + conflict-prompt (Override / Force-merge) flow.
- [ ] Absorb any blocker found while splitting; anything deferred from 0.0.15–0.0.17 lands here.

---

### 0.0.20 — UX/UI (ships the working app)

Depends on 0.0.8 (error recovery UX), 0.0.9 (settings, preference storage),
0.0.10 (notification center), 0.0.11 (panel detach), 0.0.12 (project & session
management), 0.0.13 (command center), 0.0.14 (workspaces).
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
