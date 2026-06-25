## Why

The transport verb taxonomy is incomplete. `command` (request/ack), `query` (request/response), and `subscribe` (server→client stream) exist, but there is no **duplex** verb for a bidirectional session — client sends *and* receives a stream over one named lifetime. Surface (terminal) I/O is inherently duplex (keystrokes in, output out), yet it is split across two unrelated verbs: `subscribe` (output, via the surface channel) + `surface_input`/`surface_resize` (input, hand-written off-bus commands). This split has no unifying client handle, and it has no natural mapping for the expected web/server host, where a duplex session is exactly one WebSocket.

Add `channel` as the duplex verb, declared by a `transport_channel!` macro, and unify surface I/O onto it: one session = send (input) + receive (output) + close.

## What Changes

- Add the **`channel` (duplex) verb**: a named session where the client provides a receive sink (its `tx`) and a send path. Receive reuses the `stream-subscription` machinery (key-scoped registry + `ChannelSink`) built in `bus-subscribe-streams`; send reuses the off-telemetry runtime-write path (`surface_input`'s `cx` write — no keystroke logging).
- Add a **`transport_channel!` macro** generating **two shims** for one endpoint (the Tauri-blessed duplex shape — verified against Tauri docs + maintainer guidance; `ipc::Channel` is send-only, so client->backend is `invoke`, and there is no native duplex primitive):
  - **open** — hands the backend the receive `ipc::Channel` + params, registers the receive sink (reuses the `stream-subscription` registry), starts the session;
  - **send** — ONE command carrying a tagged client->backend message (`Input(bytes)` | `Resize{cols,rows}` | `Close`); the input variant uses a **Tauri Raw Request** (raw `Uint8Array` body, no per-message serialization, batchable) and writes off-telemetry to the runtime port (no keystroke logging); `Close` tears down.
- Add a **duplex client binding**: `openChannel(...) -> { onmessage, send, resize, close }` — one handle over the receive `ipc::Channel` + the tagged send `invoke`. The two-shim split is hidden behind the one handle.
- **Migrate surface I/O onto `channel`** — **supersedes** the surface `subscribe` + `surface_input`/`surface_resize` shape from `bus-subscribe-streams`: `surface_create` + `surface_input` become one `surface` channel (open + send); `surface_resize` becomes a channel control send. The receive/output infrastructure (registry, `ChannelSink`, `SubscribeSurface`) is reused, not rebuilt.
- **`subscribe` stays** for genuinely output-only streams (e.g. the future logs follow). `command`/`query` unchanged.
- **Host mapping (verified):** desktop realizes `channel` with `ipc::Channel` (receive) + raw-byte `invoke` (send) — this is Tauri's recommended pattern, not a workaround; Tauri has no native duplex/inbound-stream primitive, and maintainers explicitly discourage a local socket/WebSocket on desktop unless IPC overhead is *measured*. The server host maps `channel` natively to one WebSocket (send + reply on one socket). The `channel` verb + the one client handle make the transport difference invisible to callers, so a future desktop->WS convergence (if ever justified) needs no caller changes.

**BREAKING (pre-v1, internal/IPC seam):** the surface transport shape changes from `surface_create`+`surface_input` to the `surface` channel. Renderer-facing call sites move to the duplex binding (additive binding lands first; the Phase-1 UI migration adopts it).

## Out of scope (follow-on)

- The server/web host WebSocket adapter for `channel` (the verb + port enable it; building it is later).
- Dissolving the remaining straggler/host commands (`window`/`menu`/`boot`/`supervisor` stay host — no verb dissolves them; the clean B stragglers can be a small separate cleanup).
- The non-blocking-I/O sweep and logs-stream follow-ons (tracked separately).

## Capabilities

### New Capabilities

- `duplex-channel`: the `channel` verb — a bidirectional session combining a client-provided receive sink with an off-telemetry send path, its open/send/close lifecycle, the transport-macro contract, and the host mapping (desktop `ipc::Channel`+`invoke`; server WebSocket). Defines what a duplex channel is and the boundary between its telemetered open and its off-telemetry send.

### Modified Capabilities

- `client-engine` (or the surface capability if one exists): surface I/O is delivered over a single duplex `channel` rather than `subscribe`+input commands. Behavior (bytes in, bytes out, ordering) is preserved; the transport shape changes. (Only include a delta if a spec actually states the surface transport shape; otherwise this is an implementation change captured by `duplex-channel`.)

## Impact

- **Code:** desktop `transport/` (`transport_channel!`), `surface_host.rs` (collapse `surface_create`/`surface_input`/`surface_resize` into the `surface` channel; keep the off-telemetry send), orchestrator surface input/runtime path (unchanged behavior, now reached via the channel send), `@tillerd/client-bindings` (`openChannel` duplex helper).
- **Dependencies:** none new.
- **ADRs:** new ADR for the `channel` duplex verb (taxonomy completion, off-telemetry send, host mapping), relating ADR-0037/0041/0042.
- **Tests:** channel open registers the receive sink; send reaches the runtime off-telemetry (no keystroke span); close tears down; surface parity (input reaches PTY, output reaches the sink) over the unified channel.
