# 0022. A Rust orchestrator crate owns the backend; TypeScript is UI and SDK

- Status: proposed
- Date: 2026-06-11

## Context

ADR-0020 defined `session` (container) and `surface` (leaf), and ADR-0021 defined
the launch spec (projects own templates; sessions are instances; launch items produce
surfaces). Both need a home: something that owns projects, sessions, surfaces, launch
execution, and persistence, and that drives the daemon and gate.

Two homes were considered:

1. A TypeScript workspace library layered above the existing TypeScript engine (the
   per-PTY surface runtime). **Rejected.**
2. A Rust orchestrator crate that also absorbs the surface runtime. **Chosen.**

Why Rust owns the backend:

- The daemon and gate are already Rust, and Rust client crates already exist —
  `daemon-pty-client`, `gate-client`, `process-launch`, `service-host`,
  `contracts-rs`. The TypeScript engine duplicates, in TS, the daemon-client and
  hook-handling logic these crates already provide.
- A future server for remote control must run the *same* backend. Orchestration in
  TypeScript forces the backend into the renderer/runtime and blocks a headless
  server, or forces a second implementation.
- The backend must run unchanged in two hosts — the desktop (Tauri) and a future
  server — without caring which.

## Decision

One Rust **orchestrator** library crate owns the backend.

### What the orchestrator owns

- **Workspace domain** — projects, sessions, surfaces, launch-spec execution
  (ADR-0021), the archive lifecycle.
- **Persistence** — rusqlite over `tillerd.db` (read and write). See ADR-0023.
- **Surface runtime** — the per-PTY proxy, hook drain, status mapping, and send queue,
  composing `daemon-pty-client` + `gate-client` + `process-launch`. Migrated from the
  TypeScript engine.
- **Agent adapter** — the `AgentDefinition` data plus the hook → status / content parse
  functions (migrated from the TS adapter and `hookEventToContent`).
- **A transport-agnostic API** — request/response methods plus outbound streams emitted
  over an `EventSink` trait the host implements.

### Embedded, not a process

The orchestrator is a **library crate**, embedded in-process by each host as a Cargo
dependency. It is not a separate process and not one of the shared singleton services.
It is the **client** of the daemon and gate (which remain separate singletons,
ADR-0020). One orchestrator instance per host process.

### Hosts are thin shells

Each host embeds the crate and binds its API to a transport:

- **desktop (Tauri)** — commands plus a `Channel` / events.
- **server (future)** — network (HTTP / WS) for remote control.

The only difference between desktop and server is the host binary; the orchestrator is
identical.

### TypeScript is UI and SDK only

- **ui** — the renderer (xterm, panels).
- **sdk** — a typed client to the orchestrator API; wire types generated from
  `contracts-rs`.
- The TypeScript **engine**, **adapter** parse logic, **platform-bun**, and the TS
  **server** are retired. This is done in one move (full-now), not phased.

### Naming

With the TS engine retired, the surface runtime is named fresh in Rust — `Session`
(container) and `Surface` (leaf). No TypeScript rename is needed: the TS types are
removed, not renamed.

### Layering

```
RUST
  orchestrator (lib crate, runtime-agnostic)
    workspace domain · persistence (rusqlite) · surface runtime · adapter
    composes: daemon-pty-client · gate-client · process-launch · contracts-rs
        ↓ embedded by
  host: desktop (Tauri)  — API over commands/events
  host: server (future)  — API over HTTP/WS  → remote control
        ▲ client of
  shared singletons: daemon, gate   (separate processes, unchanged)

TS
  ui  — renderer (xterm, panels)
  sdk — typed API client (wire types from contracts-rs)
```

## Consequences

- A single-language backend; the surface runtime is not duplicated across TypeScript
  and Rust.
- The same orchestrator runs on the desktop now and a remote server later — remote
  control is a host shell, not a re-implementation.
- TypeScript shrinks to presentation plus a generated API client; the renderer no
  longer holds backend logic or a daemon connection.
- This is a large migration: the TS engine, the adapter parse functions, and the hook /
  status mapping are reimplemented in Rust. It is the biggest single commitment in
  0.x and front-loads the risk — chosen full-now for a clean foundation before more
  services land.
- The daemon and gate are unchanged; the orchestrator is their client.
- This supersedes the earlier workspace-library direction previously recorded under
  this ADR number.
- The decision constrains the 0.x implementation but ships no code itself. Rollback is
  reverting this file.
