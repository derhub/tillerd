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
> first-run UX, settings + secrets, and a UX/UI pass complete the working app. **0.x is terminal-only**: the agent surface
> (built in 0.0.3) was removed in the launch-execution cut and is deferred to **1.0.0**
> ([ADR-0027](./adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)).
> After the working app, **0.1.x extends** — diff surface, placement geometry, workflow
> library, container backend, web remote — ordered cheapest-first on the frozen seams;
> **1.0.0** is the stable horizon and ships distribution. See ADRs
> [0020](./adr/0020-session-is-a-per-context-term-and-desktop-groups-surfaces.md)–[0027](./adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)
> for the workspace model and the 0.0.x build. See [CHANGELOG](../CHANGELOG.md).

---

## 0.0.x — Working app

The Rust inversion, then everything a daily-usable app needs. The line ends with the
working app shipping at **0.0.10**.

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
  launch spec ([ADR-0030](./adr/0030-panels-bind-surfaces-by-placement.md)). The
  panel-surface seam freezes here; the terminal revisit already shipped is the first
  slice. Sizes / nested splits stay 0.1.x (additive geometry).
- [x] `correlation_id` threaded across hops in structured logs — the log-viewer
  (0.0.7), health surfacing (0.0.8), and every later feature join records on it.
- [x] Design tokens: apply [`DESIGN.md`](../apps/ui/DESIGN.md) across the existing
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

- [ ] Per-service health indicators (gate / daemon) with failure surfacing.
- [ ] First-run / onboarding: services down, version out of range, fresh-machine setup.

### 0.0.9 — Settings and secrets

- [ ] Global settings: theme, default command library / default template.
- [ ] Per-project overrides: launch template, project env.
- [ ] Env secrets via the OS keychain; `secret_ref` stores handles only (no plaintext).
- [ ] Window state restore: size / position persisted, restored on relaunch.

### 0.0.10 — UX/UI (ships the working app)

- [ ] Interaction polish: projects / sessions navigation, empty states, pane error /
  failure states.
- [ ] better bottom and top toolbar buttons
- [ ] Motion: 60fps transitions for surface lifecycle (create / destroy); layout changes
  (adding / removing surfaces).
- [ ] Polish: icons, spacing, density, typography, popups, menus, top bar toolbox, bottom bar
  style; panel titles (session name + surface kind + running time); status badges (starting,
  running, failed); terminal font and color scheme; drag-and-drop rearrangement of surfaces;
  panel resizing (drag to resize, double-click to reset); terminal copy/paste; keyboard
  shortcuts for common actions (new project/session/surface, close surface, switch session, ...).
- [ ] Light-mode coverage: component-level appearance verified and documented (tokens
  landed in 0.0.6).
- [ ] Accessibility: keyboard navigation, screen reader labels, color contrast.
- [ ] Cross-platform polish: platform-specific UI conventions and behaviors (e.g. macOS
  traffic lights, Linux title bars) respected and tested on each OS.
- [ ] Performance: 60fps - smooth interactions and low resource usage, even with multiple
  sessions and surfaces; profiling and optimization as needed.
- [ ] E2E test coverage for all flows and platforms, integrated into CI.
- [ ] UX/UI review and iteration cycle: identify pain points, and continuously
  refine the UX.
- [ ] Final coherence pass across all surfaces.

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
