# Roadmap

Status legend: `- [ ]` planned · `- [x]` done · `[WIP]` in progress · `[HELP]` wants
input. Within a milestone, items are ordered most to least foundational. Every version
is a small, demoable step; nothing is shortcut to a `.0` bucket.

> Status: 0.0.x, pre-release. Core components are scaffolded — PTY daemon, gate,
> desktop shell — but the app does not yet work end-to-end. The **0.0.x** line builds
> the foundation (a Rust orchestrator, persistence, the surface runtime). The first
> complete working release ships at the **end of the 0.1.x** line. **0.2.x** adds more
> services; **1.0.0** is the stable horizon. See ADRs
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

- [ ] Surface runtime in Rust — per-PTY proxy, status, send-queue over
  `daemon-pty-client`; TS engine retired.
- [ ] Terminal surface — create a session (Unfiled project) with a terminal surface;
  xterm streams via the API / `EventSink`.
- [ ] Persist + resume — a `surface` row; reconnect to the daemon by `surface_id` after
  restart.

### 0.0.3 — Agent surface and hook path

The agent runs as a surface with live status and content; the gate's hook fan-out reaches
the UI.

- [ ] Adapter in Rust — `AgentDefinition` + hook → status / content parse; TS adapter-parse
  retired.
- [ ] Hook-event path — orchestrator subscribes to the gate fan-out, routes by `surface_id`.
- [ ] Agent surface UI — terminal pane + status badge (`IDLE | WORKING | WAITING_INPUT |
  DONE | crashed`) + content stream + failure states (gate / daemon / agent down).
- [ ] Idempotent setup — agent hook install wired into spawn; cleanup on uninstall;
  coexist with the user's own hooks.

### 0.0.4 — Projects and sessions (the container)

Projects group sessions; sessions group surfaces; both persist and survive restart.

- [ ] Projects — create `blank` / `local-dir` / `git-repo` / `git-worktree`; name
  inference + custom + rename; list / open; Unfiled seeded.
- [ ] Sessions — container CRUD; title inference (agent title | branch | both) + custom;
  add / remove surfaces; resume after restart.
- [ ] Layout persistence — panel tree (`layout_json`) saved per session; restored on resume.
- [ ] Archive — `deleted_at` soft-delete (cascades to surfaces); hard-delete; worktree
  directory kept.

### 0.0.5 — Launch system

Declarative startup: a session is an instance of a project's launch template.

- [ ] Launch spec — versioned JSON blob; lazy migration (ADR-0021).
- [ ] Command library — prebuilt (login shell, agent CLI) + user-added.
- [ ] Launch items — target, placement (named regions), command / args / env, pre / post
  scripts, auto-spawn, worktree step (create → cd → run).
- [ ] Templates → instances — a project template instantiates a session's surfaces; the
  session may diverge.
- [ ] Worktrees — owned by a project; created by the worktree step.

---

## 0.1.x — Working workspace (first complete release)

Harden the working slice into a shippable workspace on a standardized foundation. The
complete release ships at **0.1.7**.

### 0.1.0 — One service contract

- [ ] Solidify `service-host`: lifecycle (start / ready / drain / stop), discovery
  (socket / manifest), health (ADR-0019), identity / version.
- [ ] Gate + daemon conform; future services inherit the contract.

### 0.1.1 — Observability

- [ ] `correlation_id` threaded across hops in structured logs.
- [ ] Log-viewer surface in the desktop.

### 0.1.2 — Secrets and settings

- [ ] Env secrets via the OS keychain; `secret_ref` stores handles only (no plaintext).
- [ ] Settings: global (default agent, theme, default command library / template) +
  per-project.

### 0.1.3 — Health and first-run UX

- [ ] Per-service health indicators (gate / daemon) with failure surfacing.
- [ ] First-run / onboarding: agent binary missing, version out of range, services down.

### 0.1.4 — SDK wire types

- [ ] Generate the TS SDK wire types from `contracts-rs` (single source of truth).

### 0.1.5 — Daemon drain-and-restart upgrade

- [ ] Replace fd-handoff with a simpler planned-upgrade path. Deferred: the 0.0.x Rust
  inversion retires the TS-engine handoff this targets, so the change is re-authored
  against the orchestrator + `daemon-upgrade` / `rust-pty-daemon` specs once 0.0.x lands.

### 0.1.6 — Desktop end-to-end test

- [ ] Tauri-driving harness: embed `tauri-plugin-webdriver` (test-gated) + drive with `tauri-webdriver`
  (Choochmeque) over W3C WebDriver via WebdriverIO. Cross-platform incl. macOS (WKWebView native
  APIs), so it runs locally and in CI — unlike official `tauri-driver`, which has no macOS WKWebView
  driver (tauri#7068).
- [ ] Visual test: spawn a session, assert the terminal and agent render and stream.

### 0.1.7 — Desktop distribution (ships the first complete release)

- [ ] Signed, notarized bundles (dmg / AppImage / deb) across macOS + Linux `[HELP]` (Windows?).
- [ ] Auto-update.
- [ ] Release pipeline — versioned releases + generated changelogs via changesets.

### 0.1.8 — Docs reconciliation

- [ ] README and guides match the Rust-backend, desktop-only architecture.

---

## 0.2.x — More services and extension

Prove the seams and scale beyond one agent and one host.

### 0.2.0 — Validate the extension point

- [ ] A second agent adapter built against the contract; pressure-test before relying on it.

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
- [ ] Production-ready: distribution, observability, and upgrade paths hardened.
