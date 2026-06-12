# Changelog

All notable changes to this project are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project is pre-1.0
and APIs may break between minor versions.

## [Unreleased]

0.0.6 in progress: finalize the architecture — desktop end-to-end test, service
contract on `service-host` (lifecycle / discovery / health / identity), daemon
drain-and-restart upgrade, `correlation_id` threading, and design tokens. After 0.0.6
the architecture is frozen for 0.x; later versions are additive. The working app ships
at the end of the 0.0.x line (0.0.10); 0.1.x extends it; 1.0.0 ships distribution.

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
