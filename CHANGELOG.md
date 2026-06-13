# Changelog

All notable changes to this project are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project is pre-1.0
and APIs may break between minor versions.

## [Unreleased]

Next: 0.0.9 — settings. The architecture froze at 0.0.6; every later 0.x version is
additive on those seams. The working app ships at the end of the 0.0.x line (0.0.14);
0.1.x extends it; 1.0.0 ships distribution.

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
