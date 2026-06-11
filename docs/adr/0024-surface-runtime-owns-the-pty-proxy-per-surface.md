# 0024. Surface-runtime owns one PTY proxy per surface; surface_id is the daemon session key

- Status: proposed
- Date: 2026-06-12

## Context

ADR-0022 puts the orchestrator in charge of the backend and forbids the renderer from touching
services directly; ADR-0023 splits the id model — the product `session_id` stays inside the store,
and `surface_id` is the only id shared with backends. ADR-0020 makes a desktop session a container of
surfaces and names `surface_id` the shared kernel across the daemon and gate. The detached daemon
owns pseudo-terminals, survives host restarts (ADR-0008), is PTY-only (ADR-0016), and speaks a binary
wire (ADR-0009).

0.0.2 binds these together for the first rendering surface: a terminal. Something has to own the live
link between a product `surface` and a daemon pseudo-terminal — open it, stream its bytes to the host,
take input, resize it, track its status, and resume it after the host restarts. The three candidate
homes are the renderer, the daemon, and the orchestrator. The renderer is barred from backends
(ADR-0022) and must not see the product `session_id` (ADR-0023); the daemon is generic and
surface-unaware (ADR-0016/0008). Only the orchestrator knows both the product model and the daemon.

This decision is needed now because it is the seam every later surface kind reuses — the agent
surface (0.0.3) rides the same proxy and status path.

## Decision

The orchestrator hosts a **surface-runtime** subsystem that owns exactly one PTY proxy per terminal
surface. The proxy is the sole path between a product surface and a daemon pseudo-terminal.

- **`surface_id` is the daemon session key.** The runtime opens or attaches a daemon pseudo-terminal
  using `surface_id` as the daemon-side session id, reaffirming ADR-0020's shared kernel for the PTY
  path. The product `session_id` never crosses the boundary (ADR-0023).
- **Terminal surfaces are durable and resume by reattach.** A terminal surface persists as a store
  row; on host restart the runtime reattaches the live daemon pseudo-terminal by `surface_id` rather
  than re-spawning. If the pseudo-terminal is gone, the runtime surfaces a typed error; re-spawning
  from a launch spec is out of scope here.
- **Raw bytes flow through the `EventSink`; the host binds the byte stream to a streaming channel.**
  The proxy emits exact pseudo-terminal bytes tagged with `surface_id`; no stripping or re-decode
  (ADR-0007). The host binds each surface's byte stream to a streaming-optimized, ordered channel
  rather than a general JSON event bus, preserving throughput and ordering without mangling bytes.
- **Input is queued across attach gaps with bounded backpressure.** The proxy accepts input
  immediately, flushes it in order once attached, and applies backpressure instead of buffering
  without bound (ADR-0007).
- **Terminal status is derived from the daemon signal and emitted per surface** over the `EventSink`,
  reusing the existing terminal-status contract, delivered to a subscriber on subscribe.

This ADR supersedes nothing. It refines ADR-0020/0022/0023 at the surface level and leaves ADR-0010
(in-process fds) and ADR-0011 (fd-handoff upgrade) untouched.

## Consequences

- **Easier:** one clear owner for the surface↔pseudo-terminal link; the renderer and daemon stay
  within their existing boundaries; live session continuity across a host restart falls out of the
  detached daemon for free; the agent surface reuses the same proxy and status path.
- **Harder / costs:** `surface_id` is overloaded as the daemon's session key, coupling the product id
  to the daemon registry (intended by ADR-0020, but a real coupling). Lossless byte framing over a
  serialized event channel adds size and CPU on high-throughput output. A pseudo-terminal that exits
  between restart and reattach cannot resume and must take a typed-error path; re-spawn waits for the
  launch system.
- **Neutral:** status reuses the terminal-status contract unchanged; in-process fds and fd-handoff
  (ADR-0010/0011) are unaffected — their relaxation is a separate, deferred decision.
