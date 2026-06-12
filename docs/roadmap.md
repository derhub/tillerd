# Roadmap

Status legend: `- [ ]` planned · `- [x]` done · `[WIP]` in progress · `[HELP]` wants
input. Within a milestone, items are ordered most to least foundational. Every version
is a small, demoable step; nothing is shortcut to a `.0` bucket.

> Status: 0.0.x, pre-release. Core components are scaffolded — PTY daemon, gate,
> desktop shell — but the app does not yet work end-to-end. The **0.0.x** line builds
> the foundation (a Rust orchestrator, persistence, the surface runtime). **0.x is
> terminal-only**: the agent surface (built in 0.0.3) was removed in the launch-execution
> cut and is deferred to **1.0.0** ([ADR-0027](./adr/0027-zero-x-is-terminal-only-agent-surface-deferred.md)).
> The first complete working release ships at the **end of the 0.1.x** line. **0.2.x**
> adds more services; **1.0.0** is the stable horizon. See ADRs
> [0020](./adr/0020-session-is-a-per-context-term-and-desktop-groups-surfaces.md)–[0023](./adr/0023-workspace-data-model-and-two-level-id.md)
> and [`roadmap-plan.md`](./roadmap-plan.md). See [CHANGELOG](../CHANGELOG.md).

---

## 0.0.x — Foundation

The Rust inversion and a working vertical slice.

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
hook → status/content routing, status-badge UI, idempotent hook setup — returns in 1.0.0,
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
  worktree step (create → returns cwd, sets `worktree_id`). No pre/post/auto-spawn scripts:
  an auxiliary runner (e.g. a dev server) is an ordinary terminal item with a placement;
  closing the pane leaves the process running (soft-delete keeps the PTY).
- [x] Templates → instances — a project template instantiates a session's surfaces; the
  session may diverge. (Spec-copy on session create, executor wiring, workspace IPC
  handlers, and idempotent seed all done.)
- [x] Worktrees — owned by a project; created by the worktree step.

### 0.0.6 — Contracts and test coverage

- [ ] Solidify `service-host`: lifecycle (start / ready / drain / stop), discovery
  (socket / manifest), health (ADR-0019), identity / version.
  Gate + daemon conform; future services inherit the contract.
- [ ] Desktop E2E test: embed `tauri-plugin-webdriver` (test-gated) + drive with
  `tauri-webdriver` (Choochmeque) over W3C WebDriver via WebdriverIO. Cross-platform
  incl. macOS (WKWebView native APIs), runs locally and in CI — unlike official
  `tauri-driver`, which has no macOS WKWebView driver (tauri#7068). Visual test:
  spawn a session, assert the terminal renders and streams. (Agent render deferred
  to 1.0.0 with the agent surface.)

---

## 0.1.x — Working workspace (first complete release)

Harden the working slice into a shippable workspace on a standardized foundation. The
complete release ships at **0.1.4**.

### 0.1.0 — Observability

- [ ] `correlation_id` threaded across hops in structured logs.
- [ ] Log-viewer surface in the desktop.

### 0.1.1 — Secrets and settings

- [ ] Env secrets via the OS keychain; `secret_ref` stores handles only (no plaintext).
- [ ] Settings: global (default agent, theme, default command library / template) +
  per-project.

### 0.1.2 — Health and first-run UX

- [ ] Per-service health indicators (gate / daemon) with failure surfacing.
- [ ] First-run / onboarding: agent binary missing, version out of range, services down.

### 0.1.3 — Daemon drain-and-restart upgrade

- [ ] Replace fd-handoff with a simpler planned-upgrade path. Deferred: the 0.0.x Rust
  inversion retires the TS-engine handoff this targets, so the change is re-authored
  against the orchestrator + `daemon-upgrade` / `rust-pty-daemon` specs once 0.0.x lands.

### 0.1.4 — Desktop distribution (ships the first complete release)

- [ ] Signed, notarized bundles (dmg / AppImage / deb) across macOS + Linux `[HELP]` (Windows?).
- [ ] Auto-update.
- [ ] Release pipeline — versioned releases + generated changelogs via changesets.

### 0.1.5 — Docs reconciliation

- [ ] README and guides match the Rust-backend, desktop-only architecture.

---

## 0.2.x — More services and extension

Prove the seams and scale beyond one agent and one host.

### 0.2.0 — Validate the extension point

- [ ] Prove the surface-kind / execution-backend seam with a second implementation (the
  container backend, 0.2.1, or the diff surface, 0.2.3) — pressure-test before relying on it.
  Agent adapters are validated in 1.0.0, after the agent surface returns (ADR-0027).

### 0.2.1 — Container execution backend

- [ ] Dev-container spec / OCI runtimes behind the launch-item contract (execution-backend
  seam).

### 0.2.2 — Web remote control

- [ ] Revive the server as a Rust host embedding the same orchestrator.
- [ ] SDK over HTTP / WS; auth for remote access.

### 0.2.3 — Diff surface

- [ ] Wire the diff panel as a surface kind.

### 0.2.4 — Placement geometry

- [ ] Sizes and nested splits beyond named regions.

### 0.2.5 — Prebuilt workflow library

- [ ] Bespoke workflow sessions and dev-setup presets.

---

## 1.0.0 — Stable horizon

- [ ] Stable, versioned API and launch-spec schema.
- [ ] Extension contract (surface kinds, command library, execution backends) proven by
  real second implementations.
- [ ] Cross-platform desktop with a polished, stable UX and solid performance.
- [ ] Agent as a first-class surface kind, with a rich status model and content stream over the
  gate's hook fan-out (deferred from 0.x — ADR-0027).
- [ ] Production-ready: distribution, observability, and upgrade paths hardened.
