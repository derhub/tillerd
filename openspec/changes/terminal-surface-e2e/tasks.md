## 1. Daemon wire codec (`crates/daemon-pty-client`)

- [x] 1.1 Write failing tests: `encode_spawn`, `encode_input`, `encode_resize`, `encode_ack`, `encode_kill`, `encode_unsubscribe` produce the exact `[u32 BE len][JSON meta][0x0a?][raw body]` frames the daemon accepts, and `decode_session_frame` gains a `SpawnAck { session_id, pid }` variant — fixtures matching the `apps/daemon-pty` frame shapes (daemon-wire-protocol; ADR-0009).
- [x] 1.2 Implement the encoders + `SpawnAck` decode in `daemon-pty-client`, keeping raw body bytes intact (no re-encode); `input` carries raw bytes in the body plane.

## 2. Surface + terminal wire types (host boundary + TS mirror)

> Reconciled with reality: the orchestrator API is in-process Rust, so the cross-boundary surface
> wire types live at the host seam (mirroring the existing `StatusWire` pattern) + the SDK, not in
> `crates/contracts`. `SurfaceId` is the orchestrator domain id (`persistence`), passed to the daemon
> as the `sessionId` (ADR-0020).

- [x] 2.1 Surface command/event wire shapes keyed by `surfaceId` (`surface_create`/`input`/`resize`/`detach`, `surface://status`/`surface://exit`) defined at the host boundary and mirrored in `packages/sdk` with round-trip tests (orchestrator-core: terminal-surface lifecycle; ADR-0023 surface_id kernel).
- [x] 2.2 SDK terminal-surface client + types hand-authored in `packages/sdk` (generation deferred to 0.1.4).

## 3. Terminal surface persistence (`crates/orchestrator`)

- [x] 3.1 Write failing test: creating a terminal surface writes a durable surface row (`surface_id`, owning session, kind, cwd) readable after the store is reopened (workspace-persistence: terminal surface row persistence and resume).
- [x] 3.2 Extend the `Store` trait + `SqliteStore` + `InMemoryStore`: `get_surface`, `list_resumable_surfaces`, `update_surface_status`, mapping the existing `surface` columns (`last_status`, `cwd`, `deleted_at`) (workspace-persistence: surface row lifecycle).
- [x] 3.3 Write test: a removed (soft-deleted) surface is excluded from `list_resumable_surfaces` (workspace-persistence: removed surface is not resumed).

## 4. Daemon transport (tokio, in the surface-runtime)

- [x] 4.1 Write failing tests against a fake daemon socket: the transport connects, sends `hello` with `["snapshot"]`, asserts `hello-ack` v1, then round-trips `spawn` → `spawn-ack` keyed by `surface_id` (daemon-wire-protocol; ADR-0008/0009).
- [x] 4.2 Implement a `tokio::net::UnixStream` transport in `crates/orchestrator`: connect, handshake, framed read loop via `FrameDecoder`, framed writes; typed errors on handshake/version failure. Discover the socket via `service-host` paths (`<TILLERD_DIR>/daemon.sock`).
- [x] 4.3 Add `tokio` to `crates/orchestrator` deps and a runtime handle the surface-runtime spawns tasks on; `boot()`/`EventSink` stay synchronous.

## 5. Surface-runtime proxy (`crates/orchestrator`)

- [x] 5.1 Write failing tests (fake daemon): opening a terminal surface spawns a daemon session keyed by `surface_id`, with exactly one proxy per surface and no second proxy for the same id (surface-runtime: one PTY proxy per surface; ADR-0020/0024).
- [x] 5.2 Implement the per-surface proxy as a tokio task: spawn-or-subscribe by `surface_id`, decode `data`/`status`/`exit`, and fan raw bytes + status to the surface event sink tagged with `surface_id`, preserving control sequences unchanged (surface-runtime: outbound raw-byte streaming, terminal status emission; ADR-0007).
- [x] 5.3 Implement flow control: return credit via `ack` frames as `data` is forwarded, and apply bounded backpressure on the outbound path (surface-runtime: backpressure under load; daemon-flow-control; ADR-0007).
- [x] 5.4 Implement the input send-queue: accept input immediately, flush in arrival order once the daemon session is live, queue while spawning/attaching (surface-runtime: input send-queue).
- [x] 5.5 Implement resize: forward `resize`, and apply the latest known dimensions on attach and reattach (surface-runtime: resize propagation).
- [x] 5.6 Implement detach vs removal: detach (host shutdown / dropped client) sends `unsubscribe` and leaves the daemon session alive; removing the surface sends `kill`/`stop` and releases the proxy (surface-runtime: detach preserves the pseudo-terminal; removal terminates it).

## 6. Resume by surface identifier

- [x] 6.1 Write failing test: after a simulated host restart, a persisted surface whose daemon session is alive re-subscribes by `surface_id` without re-spawning (surface-runtime: reconnect by surface identifier).
- [x] 6.2 Implement boot-time resume: read resumable surface rows and `subscribe` each by `surface_id`; the daemon's snapshot/replay + `status` supplies initial paint (surface-runtime: reconnect by surface identifier; ADR-0008).
- [x] 6.3 Write test: subscribing to a missing daemon session surfaces a typed error and does not silently re-spawn (surface-runtime: pseudo-terminal gone after restart).

## 7. Orchestrator API and event streams

- [x] 7.1 Write failing tests: a create-terminal-surface method returns a `surface_id` and starts the proxy; input and resize route to the proxy; output and status emit as surface events tagged with `surface_id` (orchestrator-core: terminal-surface lifecycle).
- [x] 7.2 Implement the API methods (create-terminal-surface in a session, send input, resize), resolving session-or-Unfiled, writing the surface row, and starting the runtime proxy (orchestrator-core; workspace-persistence: default project).
- [x] 7.3 Extend the host event surface with surface byte + status events distinct from the boot `Status` stream; emit them from the runtime (orchestrator-core: output and status as events; ADR-0022).
- [x] 7.4 Write test: creating a terminal surface without an explicit project places its session under the seeded Unfiled project (orchestrator-core: default project when none given).

## 8. Desktop host transport (`apps/desktop/src-tauri`)

- [x] 8.1 Add commands for create-terminal-surface, input, and resize that call the orchestrator, and bind each surface's byte stream to a per-surface `tauri::ipc::Channel<Vec<u8>>` (mirroring the existing `bridge.rs` byte-channel pattern); status changes ride the event channel (design: desktop host binds the byte stream to a streaming channel; ADR-0024).
- [x] 8.2 Verify the orchestrator (not the renderer bridge) owns the daemon connection for terminal surfaces (design: the orchestrator owns the daemon socket).

## 9. SDK terminal-surface client (`packages/sdk`)

- [x] 9.1 Write failing tests: the SDK create returns a `surface_id`; subscribe delivers raw bytes over the surface channel and status; input and resize forward keyed by `surface_id` and never open a daemon connection (sdk-orchestrator-client: typed terminal-surface client).
- [x] 9.2 Implement the typed terminal-surface client over the orchestrator commands + the per-surface byte channel, with wire types centralized in one module (sdk-orchestrator-client).

## 10. UI terminal pane (`apps/ui`)

- [x] 10.1 Wire `DesktopTerminalPane` to accept a `surface_id`, attach through the SDK to the surface byte channel, render raw bytes into the xterm pane preserving ANSI, clear and reattach on `surface_id` change, and tear down cleanly on unmount leaving the daemon session alive (ui-terminal-pane: session-scoped terminal connection; terminal output rendering).
- [x] 10.2 Send a resize for the surface to the orchestrator via the SDK when the container resizes (ui-terminal-pane: resize propagates).
- [x] 10.3 Drive the connection status indicator and manual reconnect from the surface attachment state (ui-terminal-pane: connection status indicator).

## 11. Retire the engine for the terminal path

- [x] 11.1 Route the desktop terminal through the orchestrator surface-runtime; remove the in-renderer engine + WebSocket-to-server terminal transport from the desktop terminal flow (desktop-engine-runtime: desktop terminal I/O flows through the surface-runtime; ui-terminal-pane).
- [x] 11.2 Verify no in-renderer engine carries the terminal pseudo-terminal I/O on the desktop host (desktop-engine-runtime: engine is off the terminal path).

## 12. Verification

- [ ] 12.1 Create a session in the Unfiled project with a terminal surface; the terminal renders and streams live pseudo-terminal bytes through the orchestrator API / surface channel, not the engine (acceptance 1).
- [ ] 12.2 Confirm the surface-runtime proxies one daemon session per surface over the tokio transport, exposes status, and queues sends with flow control (acceptance 2).
- [ ] 12.3 Confirm a surface row persists and, after a host restart, the runtime re-subscribes by `surface_id` and the live session reattaches with snapshot paint (acceptance 3).
- [ ] 12.4 Confirm the engine path for terminal surfaces is off on the desktop host (acceptance 4).
- [ ] 12.5 `cargo test` passes for `daemon-pty-client`, `contracts`, `orchestrator`; `turbo test`, `turbo lint`, `turbo build` pass for the touched packages; `cargo clippy --all-targets -- -D warnings` is clean (testing gate; rust-best-practices).
