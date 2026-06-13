## Why

Implements roadmap **0.0.2 — "Terminal surface, end-to-end"** (the second version of the 0.0.x
Foundation line).

0.0.1 stood up the Rust orchestrator: it boots, supervises the daemon and gate, opens the durable
product store, and reaches a `ready` state the SDK observes — but nothing renders, and the store's
`surface` rows are unpopulated by design (deferred to 0.0.2+). The terminal a user actually sees
still depends on the retiring in-renderer TypeScript engine path. This change builds the first
vertical slice that renders: a single terminal surface created, streamed, persisted, and resumed
end-to-end through the Rust stack, retiring the engine for the terminal path. It proves the
orchestrator -> daemon seam the rest of 0.0.x stands on, and is the substrate 0.0.3's agent surface
reuses.

## What Changes

- **Introduce a surface-runtime subsystem in the orchestrator.** It owns one PTY proxy per surface
  over `daemon-pty-client`: open or attach a daemon PTY keyed by `surface_id`, stream raw PTY bytes
  outbound through the host `EventSink`, accept input through a per-surface send-queue, propagate
  terminal resize, and track and emit terminal status. Raw bytes pass end-to-end — no ANSI
  stripping, no UTF-8 re-decode across the hop (ADR-0007).
- **Extend the orchestrator API with the terminal-surface lifecycle.** Create a terminal surface in
  a session (a session under the seeded Unfiled project when none is given), attach to its outbound
  byte and status streams, send input, and resize — request/response methods plus the outbound
  event streams, transport-agnostic over the `EventSink`.
- **Make the terminal `surface` row concrete.** The store persists a terminal surface — `surface_id`,
  its session reference, kind, and the launch metadata needed to re-spawn or re-attach — so a
  surface outlives a host restart and the runtime reconnects to the running daemon by `surface_id`.
  Schema lands via a lazy migration (ADR-0023). `surface_id` remains the only id shared across
  backends.
- **Make the SDK a typed terminal-surface client.** It creates a terminal surface, subscribes to the
  byte and status event streams, sends input, and resizes — over the orchestrator API through the
  host transport.
- **Render the terminal through the orchestrator. BREAKING.** The terminal pane in the renderer
  renders bytes streamed from the orchestrator `EventSink` keyed by `surface_id` and sends
  input/resize through the SDK. The engine-era WebSocket-to-server terminal transport is removed.
- **Retire the TypeScript engine for the terminal surface. BREAKING.** Desktop terminal PTY I/O no
  longer routes through the in-renderer engine; it flows through the orchestrator surface-runtime.

## Capabilities

### New Capabilities

- `surface-runtime`: the orchestrator subsystem that owns one daemon-PTY proxy per surface over
  `daemon-pty-client` — open/attach by `surface_id`, outbound raw-byte streaming over the
  `EventSink`, a per-surface input send-queue, resize propagation, terminal-status tracking and
  emission, and reconnect-by-`surface_id` after a host restart.

### Modified Capabilities

- `orchestrator-core`: the transport-agnostic API surface gains the terminal-surface lifecycle —
  create-in-session, attach, send input, resize — and the matching outbound PTY-byte and
  terminal-status event streams.
- `workspace-persistence`: the terminal `surface` row becomes a populated, durable entity
  (`surface_id`, session reference, kind, launch/attach metadata) supporting persist-and-resume by
  `surface_id`; a session is created under the seeded Unfiled project.
- `sdk-orchestrator-client`: the SDK adds a typed terminal-surface client — create, subscribe to
  byte and status streams, send input, resize — over the orchestrator API.
- `ui-terminal-pane`: the pane connects by `surface_id` and renders bytes delivered over the
  orchestrator `EventSink`, sending input/resize through the SDK; the WebSocket-to-server terminal
  transport is removed.
- `desktop-engine-runtime`: desktop terminal PTY I/O no longer runs through the in-renderer engine;
  the terminal flows through the orchestrator surface-runtime.

## Impact

- **Crates:** the orchestrator crate gains a surface-runtime module (per-surface PTY proxy,
  send-queue, status fan-out), the terminal-surface API methods/events, and persistence of surface
  rows. It composes `daemon-pty-client` as the PTY transport and reuses the `daemon-wire-protocol`,
  `frame-codec`, and `terminal-status` contracts unchanged (the runtime translates daemon
  terminal-status frames into surface-scoped status events). `contracts` gains the surface and
  terminal request/event types.
- **SDK / UI:** `packages/sdk` adds the terminal-surface client; `apps/ui` rewires `ui-terminal-pane`
  to the orchestrator `EventSink` and SDK. The engine-era terminal PTY path in `packages/engine` /
  `apps/server` is no longer on the desktop terminal flow.
- **Data:** a terminal `surface` row schema added by a lazy migration (ADR-0023); `surface_id` is the
  cross-backend kernel already mandated by `workspace-persistence`.
- **ADRs:** honors ADR-0022 (orchestrator owns the backend; transport-agnostic API + `EventSink`),
  ADR-0023 (one product store, two-level id, lazy migration), ADR-0020 (session = container of
  surfaces; `surface_id` shared kernel), ADR-0008/0009/0016 (daemon is PTY-only over the binary wire),
  ADR-0007 (reliability: send-queue/backpressure, typed errors, raw bytes, resume). A new ADR records
  the surface-runtime ownership seam — the orchestrator owns the per-PTY proxy and `surface_id` is the
  shared key to the daemon.
- **Out of scope:** the agent surface and hook path (0.0.3), project/session CRUD beyond the seeded
  Unfiled default (0.0.4), the launch system (0.0.5), and daemon drain/upgrade (0.1.5). Engine-era
  `session-persistence` keeps its session-id-keyed reconnect; this change adds surface-keyed
  persist/resume and does not rewrite it.
