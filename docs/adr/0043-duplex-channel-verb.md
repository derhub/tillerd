# 0043. The duplex `channel` verb: bidirectional session, off-telemetry send, host-mapped transport

- Status: accepted
- Date: 2026-06-25
- Relates: ADR-0037 (zero-copy event dispatch), ADR-0041 (tower bus middleware), ADR-0042 (client-provided stream subscriptions)

## Context

The transport taxonomy had `command` (request/ack), `query` (request/response), and `subscribe` (server->client stream), but no duplex verb for a bidirectional session. Surface (terminal) I/O is inherently duplex yet was split across `subscribe` (output) and hand-written `surface_input`/`surface_resize` (input), with no unifying handle and no natural host mapping.

Verified against Tauri 2.11 docs + maintainer guidance: `tauri::ipc::Channel<TSend>` is send-only (backend->client); Tauri has no native duplex/bidirectional IPC primitive; client->backend is `invoke`, optionally as a Raw Request carrying batchable raw bytes; the event system is bidirectional but global and JS-eval-based (unsuitable for bulk); a local socket/WebSocket is explicitly discouraged unless IPC overhead is measured. The expected pre-v1 web/server host maps a duplex session natively to one WebSocket.

## Decision

Add `channel` as the duplex transport verb.

- **A `channel` is a named bidirectional session:** open (client provides a receive sink + params) -> send (client->backend messages) + receive (backend->client stream) -> close. One client handle (`openChannel`) represents the whole session.

- **Two shims on desktop, hidden behind one handle.** `ipc::Channel` is send-only, so the floor is two `#[tauri::command]`s: **open** (hands the receive `ipc::Channel` + params, registers the receive sink) and **send** (one tagged command `Input | Resize | Close`). This is Tauri's recommended pattern, not a workaround. The two-shim split does not leak into the caller's model.

- **Send is off the telemetry path.** `Input` (and `Resize`) bypass the bus dispatch pipeline and write directly to the runtime port (as input does today), arriving as a Tauri Raw Request (raw bytes, batchable). No span, log, or recording layer ever sees the payload — preserving the keystroke-never-logged invariant (the reason the `Io` bus trait was removed in ADR-0037). Only `open` and `Close` ride the bus and are telemetered.

- **Receive reuses stream-subscription (ADR-0042).** The open command registers the client's `ChannelSink` into the key-scoped registry; the pump delivers borrowed frames zero-copy. No new receive machinery; the single owned copy is at the host boundary.

- **Surface I/O unifies onto one `channel`.** `surface_create` + `surface_input` + `surface_resize` collapse into one `surface` channel (open + tagged send), superseding the surface transport from ADR-0042 (whose registry/`ChannelSink`/`SubscribeSurface` are reused as the receive half). `subscribe` is retained for output-only streams.

- **Host-mapped transport.** Desktop realizes `channel` with `ipc::Channel` (receive) + raw-byte `invoke` (send); the server host maps it natively to one WebSocket. The verb + the one handle make the transport difference invisible to callers, so a future desktop->WebSocket convergence (only if overhead is ever measured) needs no caller change. No local WebSocket on desktop now.

- **`command`/`query` are unchanged.** Unary stays request/response; only streaming/duplex uses sinks. The arity split (0/1/N/duplex) maps cleanly to both hosts.

## Consequences

- The taxonomy is complete: `command`, `query`, `subscribe`, `channel`. Surface I/O is one session with one client handle.
- The receive half is reused from ADR-0042; only the send/open wiring is new. Surface is re-tooled a second time (subscribe -> channel); the churn is bounded to input/open.
- The keystroke-never-logged invariant is preserved structurally (send is off the bus) and guarded by a test.
- The server host gains a clean WebSocket mapping for streaming; desktop stays on native Tauri IPC, per maintainer guidance.
- One owned copy per received frame at the process boundary remains (ADR-0042); the internal path stays zero-copy.
