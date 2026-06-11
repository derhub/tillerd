## Context

0.0.1 stood up the orchestrator (ADR-0022): it boots, supervises the detached daemon and gate,
opens the durable product store (ADR-0023), and reaches `ready`. The store seeds the Unfiled project
and reserves a `surface` entity, but no surface rows are written and nothing renders. The terminal a
user sees still rides the in-renderer engine path.

This change makes one terminal surface stream end-to-end through the Rust stack and retires the
engine for that path. The detached daemon already owns pseudo-terminals and survives host restarts
(ADR-0008), speaks a length-prefixed binary wire (ADR-0009), holds the PTY master fds in its main
process (ADR-0010), and is PTY-only (ADR-0016). The orchestrator reaches it through the existing
`daemon-pty-client` crate. What is missing is the seam that binds a product `surface` to a daemon
pseudo-terminal, streams it to the host, and resumes it after a restart.

In-force decisions this design must stay coherent with: ADR-0007 (reliability: backpressure, typed
errors, raw bytes, graceful shutdown), ADR-0008/0009/0010/0016 (daemon ownership, wire, fds,
PTY-only), ADR-0019 (in-process health), ADR-0020 (a desktop session is a container of surfaces;
`surface_id` is the shared kernel), ADR-0022 (orchestrator owns the backend; transport-agnostic API
+ `EventSink`), ADR-0023 (one product store, two-level id, lazy migration). ADR-0011 (fd-handoff
upgrade) and ADR-0010 (in-process fds) remain in force — relaxing them belongs to the deferred
0.1.5 drain-and-restart change, not here.

## Goals / Non-Goals

**Goals:**

- A surface-runtime subsystem in the orchestrator that owns one PTY proxy per terminal surface over
  `daemon-pty-client`: stream raw bytes out, queue input in, propagate resize, track and emit status.
- Persist a terminal `surface` row and resume it after a host restart by reattaching to the live
  daemon pseudo-terminal, keyed by `surface_id`.
- Render one terminal surface in the desktop renderer through the orchestrator API / `EventSink` and
  the SDK, with the engine off the terminal path.

**Non-Goals:**

- The agent surface and hook path (0.0.3); re-spawning a dead pseudo-terminal from a launch spec
  (0.0.5); daemon drain/upgrade and any ADR-0010/0011 relaxation (0.1.5); the web/network transport
  (0.2.2); multi-surface layout and project/session CRUD beyond the seeded Unfiled default (0.0.4);
  subprocess-per-session crash isolation.

## Decisions

**The PTY proxy lives in the orchestrator, not the daemon or the renderer.** The surface-runtime is
an orchestrator subsystem that owns one proxy per surface. _Alternatives:_ (a) the renderer connects
to the daemon directly — rejected: violates ADR-0022 (orchestrator owns the backend) and ADR-0023's
two-level id (the product `session_id` must not leave the orchestrator; only `surface_id` crosses to
backends); (b) the proxy lives in the daemon — rejected: the daemon stays generic and PTY-only
(ADR-0016/0008), unaware of surfaces. The orchestrator is the only component that knows both the
product model and the daemon.

**`surface_id` is the daemon's session key.** Per ADR-0020 the surface identifier is the id reused
across the daemon and gate. The runtime opens/attaches a daemon pseudo-terminal using `surface_id`
as the daemon-side session id, so the product `session_id` never crosses the boundary. _Alternative:_
mint a separate daemon id and map it in the store — rejected as redundant; the shared-kernel rule
already gives one stable id.

**Resume reattaches the live pseudo-terminal; it does not re-spawn.** On host restart the runtime
reads persisted surface rows and reattaches each to its still-running daemon pseudo-terminal by
`surface_id` (the daemon is detached and outlives the host, ADR-0008). If the pseudo-terminal is
gone, the runtime surfaces a typed error; re-spawning from a launch spec is 0.0.5. _Alternative:_
always re-spawn on restart — rejected: loses live session continuity that the detached daemon already
provides.

**Raw bytes pass through the `EventSink`; the desktop host binds the byte stream to a streaming
channel.** The orchestrator emits exact pseudo-terminal bytes to the `EventSink` tagged with
`surface_id`; no stripping or re-decode (ADR-0007). The desktop host binds each surface's byte stream
to a per-surface streaming channel (`tauri::ipc::Channel`) — the ordered, high-throughput primitive
Tauri uses internally for child-process output — not the general event system. _Alternative:_ base64
chunks over the event system — rejected on current Tauri guidance: the event system carries JSON
payloads and is "not designed for low latency or high throughput," while channels are built for
exactly this streaming case. The `EventSink` contract is unchanged; the channel is purely the desktop
binding. Lower-rate terminal-status changes stay on the event system.

**No new crate or package.** The surface-runtime is a module inside `crates/orchestrator`; surface +
terminal API/event types extend `crates/contracts`; the daemon wire encoders extend
`crates/daemon-pty-client` (codec colocated with its existing `encode_hello`/`encode_subscribe`, not a
split); the client extends `packages/sdk`; the pane and host wiring extend `apps/ui` and
`apps/desktop/src-tauri`. `tokio` is added as a dependency (not a workspace crate). A new crate is
introduced only with an explicit, approved reason — none exists here.

**Input is queued during attach, with bounded backpressure.** The proxy accepts input immediately
and flushes it in order once attached (open or reattach), and applies backpressure rather than
buffering without bound when the pseudo-terminal cannot keep up (ADR-0007). This keeps keystrokes
ordered across an attach gap without unbounded memory.

**Terminal status is derived from the daemon signal and emitted per surface.** The runtime maps the
daemon's `status` frames to surface-scoped status events on the `EventSink`, delivered to a subscriber
on subscribe (current status) independent of the byte stream, reusing the existing `terminal-status`
contract.

**Engine retirement is scoped to the desktop terminal path.** `ui-terminal-pane` stops using the
engine-era WebSocket-to-server transport and attaches through the orchestrator over the native
transport; the engine no longer carries terminal pseudo-terminal I/O on desktop. `apps/server` and
`packages/engine` are not deleted wholesale here — the web transport revival is 0.2.2.

### Implementation reality (post-recon)

Recon of the running code (not the spec's idealized shape) fixes the build to these facts:

**`tokio` is the IO model.** The surface-runtime runs each surface's daemon connection as a tokio
task over `tokio::net::UnixStream`; the orchestrator owns a tokio runtime handle. _Alternative:_
blocking threads (matches the rest of the stack today) — not chosen; tokio is the selected model for
the surface IO path. `boot()`/`EventSink` stay synchronous; the runtime bridges async↔sync at the
sink boundary.

**The orchestrator owns the daemon socket.** Today the renderer-driven bridge (`bridge.rs`
`daemon_connect`) owns the daemon connection. For terminal surfaces that moves into the orchestrator:
the surface-runtime opens its own `UnixStream` to `<TILLERD_DIR>/daemon.sock` (discovered via the
`service-host` paths), one connection per surface for 0.0.2 (the daemon multiplexes by `sessionId`;
per-surface connections are simplest and isolate lifecycle — multiplexing is a later optimization).

**The daemon wire is already defined; the client crate only lacks encoders.** `crates/daemon-pty-client`
has the framing (`[u32 BE len][JSON meta][0x0a?][raw body]`), `encode_hello`, `encode_subscribe`, and
`decode_session_frame`. This change adds `encode_spawn`/`encode_input`/`encode_resize`/`encode_ack`/
`encode_kill`/`encode_unsubscribe` and a `SpawnAck` decode variant, matching the daemon's
`apps/daemon-pty` frame shapes. `surface_id` is sent verbatim as the daemon `sessionId` on `spawn`
(the daemon accepts a client-supplied id and echoes `spawn-ack`), so the shared-kernel rule needs no
mapping table.

**Flow control is honored.** The daemon meters output by credit; the proxy returns credit via `ack`
frames as it forwards `data`, matching the existing engine proxy and `daemon-flow-control` (ADR-0007
backpressure). The input send-queue and outbound credit are the two backpressure points.

**Initial paint is the daemon snapshot.** The proxy advertises `capabilities: ["snapshot"]` in
`hello`; on `subscribe` the daemon sends a `snapshot` (VT cell grid) — or raw replay for
non-snapshot — followed by a `status` frame. That snapshot is the scrollback source on attach and
resume; no separate reconstruction is built here.

## Risks / Trade-offs

- **Per-surface daemon connection** → one `UnixStream` per surface is simpler but does not share the
  socket. Fine at 0.0.2's single-terminal scale; multiplex by `sessionId` over one connection later if
  surface counts grow.
- **Async↔sync bridge at the `EventSink`** → the surface-runtime is async (tokio) while `EventSink` is
  sync; care is needed not to block the runtime in `emit`. Mitigation: `emit` only hands bytes to the
  host channel (non-blocking); no heavy work on the sink thread.
- **Reattach race: the pseudo-terminal exits between restart and reattach** → the surface cannot
  resume. Mitigation: the typed-error path reports the surface as not resumable rather than silently
  attaching elsewhere; re-spawn arrives with the launch system (0.0.5).
- **`surface_id` overloaded as the daemon session key** → couples the product id to the daemon's
  registry semantics. Mitigation: this is exactly ADR-0020's shared-kernel intent; the daemon treats
  it as an opaque session id.
- **Legacy requirement name retained** → `ui-terminal-pane`'s "Session-scoped terminal connection"
  now attaches by `surface_id`. Kept to avoid a rename mid-flight; revisit when a session holds
  multiple surfaces (0.0.4). Flagged in Open Questions.

## Migration Plan

- A lazy schema migration adds the terminal `surface` row (ADR-0023's migration runner keyed off the
  `meta` schema version). No data import — pre-v1, the schema starts fresh.
- Rollback: revert the binary; an older binary that does not understand the new schema version is
  refused by the existing newer-store guard (ADR-0023). Acceptable pre-v1.
- Cutover is internal: the renderer's terminal pane switches transport; no external surface or
  external consumer changes.

## Resolved Questions

- **Byte transport (RESOLVED).** Per-surface `tauri::ipc::Channel` for the byte stream on desktop,
  event system for status — see the byte-stream decision above. Not a base64-over-events path.
- **Initial-paint source on attach (RESOLVED).** The daemon's existing replay buffer supplies
  scrollback on attach; the richer `virtual-terminal-state` snapshot-on-subscribe is deferred until a
  surface needs exact state reconstruction.
- **No new crate (RESOLVED).** Modules in existing crates/packages only — see the no-new-crate
  decision above.
- **In-force ADRs (RESOLVED).** No in-force ADR is revisited. ADR-0010/0011 stay as-is; their
  relaxation is the deferred 0.1.5 change. ADR-0024 records the surface-runtime ownership seam and
  supersedes nothing.
- **Requirement rename (RESOLVED).** Keep `ui-terminal-pane`'s "Session-scoped terminal connection"
  header for 0.0.2 (the body attaches by `surface_id`); defer the rename to "Surface-scoped" to 0.0.4
  when a session holds multiple surfaces — avoids a RENAMED+MODIFIED delta on one requirement now.

## Open Questions

None — all resolved above.
