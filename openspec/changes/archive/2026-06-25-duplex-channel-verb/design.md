## Context

The transport verbs are `command`/`query`/`subscribe`; there is no duplex verb. Surface I/O is split across `subscribe` (output, the `bus-subscribe-streams` machinery) and hand-written `surface_input`/`surface_resize` (input, off-bus runtime writes). Web research (verified, Tauri 2.11): `ipc::Channel<TSend>` is send-only (backend->client); Tauri has no native duplex primitive; client->backend is `invoke`, and a Tauri **Raw Request** can carry raw bytes batchably; maintainers discourage a local socket/WebSocket unless overhead is measured. The server host (expected pre-v1) maps a duplex session natively to one WebSocket.

This change adds the `channel` duplex verb and unifies surface I/O onto it. The receive half reuses the stream-subscription registry + `ChannelSink` (ADR-0042); the send half reuses the off-telemetry runtime-write path.

## Goals / Non-Goals

**Goals:**

- A `channel` verb: open (client receive sink + params) + tagged send (`Input`/`Resize`/`Close`) + close, as one client handle.
- `transport_channel!` macro generating the two desktop shims (open, tagged send).
- `Input` send off the telemetry path (no keystroke logging); reused receive passthrough.
- Migrate surface I/O onto one channel; preserve behavior + ordering.

**Non-Goals:**

- Server/web WebSocket adapter (the verb enables it; building it is later).
- A local WebSocket on desktop (discouraged by Tauri maintainers absent measured overhead).
- Dissolving host-only commands (`window`/`menu`/`boot`/`supervisor` stay) or the non-blocking-I/O / logs-stream follow-ons.

## Decisions

### D1. Two shims on desktop, one client handle (verified Tauri-blessed shape)

`transport_channel!` generates exactly two `#[tauri::command]`s per endpoint:

- **open** `name(channel: ipc::Channel<Out>, ...params)` — builds the host receive adapter (`ChannelSink::for_channel`), `bus.execute`s the session-open command(s) that register the sink in the key-scoped registry (ADR-0042), returns the session key.
- **send** `name_send(key, msg: <Tagged>)` — one command for all client->backend messages: `Input(bytes)` | `Resize{cols,rows}` | `Close`. `Input` arrives as a Tauri Raw Request (raw `Uint8Array` body, no per-message serialization) and is written off-telemetry to the runtime port; `Resize` is an off-telemetry runtime write; `Close` is `bus.execute(UnsubscribeSurface...)`.

The client binding `openChannel(...) -> { onmessage, send, resize, close }` hides the split. Rationale: `ipc::Channel` is send-only and Tauri has no duplex primitive, so two shims is the floor; this is the recommended pattern, not a workaround.

### D2. Send is off the telemetry path

`Input` (and `Resize`) do NOT go through `bus.execute`'s tower pipeline — they call the runtime-write path directly on `cx` (as `surface_input` does today), so no span/log/recording layer ever sees the payload. This preserves the keystroke-never-logged invariant (the reason the `Io` bus trait was removed, ADR-0037). Only `open`/`Close` ride the bus (telemetered).

### D3. Receive reuses stream-subscription (ADR-0042)

The receive direction is the existing key-scoped registry + `ChannelSink`: the open command registers the client's `ChannelSink`; the pump dispatches borrowed frames to it. Zero-copy passthrough, one edge copy. No new receive machinery.

### D4. Surface migrates onto one channel; subscribe retained elsewhere

`surface_create` (subscribe) + `surface_input` + `surface_resize` collapse into one `surface` channel: open = spawn + register receive sink; send = `Input`/`Resize`/`Close`. This supersedes the surface transport from `bus-subscribe-streams` (whose registry/`ChannelSink`/`SubscribeSurface` are reused as the receive half). `subscribe` stays for output-only streams (logs follow-on).

### D5. Host mapping

Desktop: `ipc::Channel` (receive) + raw-byte `invoke` (send). Server: one WebSocket (send + reply). The `channel` verb + the one handle make the transport swap invisible to callers; no desktop WebSocket now.

## Risks / Trade-offs

- **Re-churning surface** (migrated to subscribe in `bus-subscribe-streams`, now to channel) -> wasted churn risk. Mitigation: the receive half is reused unchanged; only the input/open wiring is re-tooled; keep behavior + parity tests; wire-shape change is internal (pre-v1).
- **Stacks on two unpushed changes** -> compounding unmergeable pile. Mitigation: backend-green; push story tracked separately.
- **Off-telemetry send must stay off the bus** -> a future refactor could accidentally route input through telemetry. Mitigation: a test asserts a data send produces no span/log (the keystroke-never-logged invariant).
- **Renderer migration** -> the duplex binding lands additively; the Phase-1 UI adopts `openChannel` for surfaces. The old `surface_create`/`surface_input` wire path is replaced (renderer call sites change). Mitigation: ship the binding first; UI migration is its own step.

## Migration Plan

1. `transport_channel!` macro (open + tagged send) in `transport/`.
2. Tagged send message type + the off-telemetry `Input`/`Resize` runtime writes; `Close` -> `UnsubscribeSurface`.
3. Migrate `surface`: collapse `surface_create`/`surface_input`/`surface_resize` into the `surface` channel (open + send). Reuse `SubscribeSurface`/registry/`ChannelSink` for receive.
4. `openChannel` duplex client binding; update the surface call site path.
5. ADR-0043; parity + off-telemetry + teardown tests; backend gate.

Rollback: pre-v1 internal seam; revert restores the subscribe+input surface shape.

## Open Questions

- Exact tagged-send representation (one enum command vs a raw-body command with a control sidecar) — resolved in APPLY against the Raw Request shape; does not change the spec contract.
- Whether `Resize` is a send variant or a small separate command — APPLY; both satisfy the spec.
