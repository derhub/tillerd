## 1. Contracts and shared types

- [ ] 1.1 Define the surface + terminal contract types in `contracts`: `surface_id`, the terminal-surface kind, the create-terminal-surface request/result, the input and resize requests, and the outbound byte event and terminal-status event — all keyed by `surface_id` (orchestrator-core: terminal-surface lifecycle; ADR-0023 surface_id kernel). Hand-author the wire types (generation deferred to 0.1.4).
- [ ] 1.2 Write failing serde round-trip tests for the new contract types, using fixtures per the contracts fixtures convention (contracts: typed frame catalogue).

## 2. Terminal surface persistence

- [ ] 2.1 Write failing test: creating a terminal surface writes a durable surface row (`surface_id`, owning session reference, kind) that is readable after the store is reopened (workspace-persistence: terminal surface row persistence and resume).
- [ ] 2.2 Add the terminal `surface` row to the schema as an ordered lazy migration on the existing runner; Rust-only access, no service-local state in the store (workspace-persistence; ADR-0023 migration runner).
- [ ] 2.3 Implement Store methods behind the repository trait — insert surface, get surface by `surface_id`, list resumable surfaces, remove surface — and extend the in-memory fake Store (workspace-persistence: surface row lifecycle).
- [ ] 2.4 Write test: a removed surface is not returned by the resumable-surfaces query (workspace-persistence: removed surface is not resumed).

## 3. Surface-runtime: per-surface PTY proxy

- [ ] 3.1 Write failing tests against a fake daemon client: opening a surface establishes exactly one proxy bound to `surface_id` and one pseudo-terminal, and no second proxy exists for the same surface (surface-runtime: one PTY proxy per surface).
- [ ] 3.2 Implement the surface-runtime module in `crates/orchestrator`: a proxy that opens or attaches a daemon pseudo-terminal via `daemon-pty-client` using `surface_id` as the daemon session key (surface-runtime: one PTY proxy per surface; ADR-0020/0024).
- [ ] 3.3 Implement outbound raw-byte streaming: forward exact pseudo-terminal bytes tagged with `surface_id`, no stripping or re-decode; test that output containing control and escape sequences passes through unchanged (surface-runtime: outbound raw-byte streaming; ADR-0007).
- [ ] 3.4 Implement the input send-queue: accept input immediately, flush in arrival order once attached, queue while attaching; test ordering across an attach gap (surface-runtime: input send-queue).
- [ ] 3.5 Implement bounded backpressure on input when the pseudo-terminal cannot keep up; test it does not buffer without bound (surface-runtime: backpressure under load; ADR-0007).
- [ ] 3.6 Implement resize propagation: forward dimensions to the pseudo-terminal and apply the latest known dimensions on attach and reattach; test both (surface-runtime: resize propagation).
- [ ] 3.7 Implement terminal-status tracking: map the daemon's terminal-status frames to surface-scoped status events on the `EventSink`, and deliver the current status on subscribe; test (surface-runtime: terminal status emission; terminal-status contract).
- [ ] 3.8 Implement detach vs removal: a detach from host shutdown or a dropped client leaves the pseudo-terminal alive, while removing the surface terminates it and releases the proxy; test both (surface-runtime: detach preserves the pseudo-terminal; removal terminates it).

## 4. Resume by surface identifier

- [ ] 4.1 Write failing test: after a simulated host restart, a persisted surface whose pseudo-terminal is still alive reattaches by `surface_id` without spawning a new pseudo-terminal (surface-runtime: reconnect by surface identifier).
- [ ] 4.2 Implement boot-time resume: read resumable surface rows and reattach each proxy by `surface_id` (surface-runtime: reconnect by surface identifier; ADR-0008 detached daemon).
- [ ] 4.3 Write test: when the pseudo-terminal is gone, reattach surfaces a typed error and does not silently attach to a different pseudo-terminal (surface-runtime: pseudo-terminal gone after restart).

## 5. Orchestrator API and event streams

- [ ] 5.1 Write failing tests for the API: create-terminal-surface returns a `surface_id` and starts the proxy; input and resize route to the proxy; output and status emit as `EventSink` events tagged with `surface_id` (orchestrator-core: terminal-surface lifecycle).
- [ ] 5.2 Implement the API methods (create-terminal-surface in a session, send input, resize), wiring session-or-Unfiled resolution to the surface row and the surface-runtime proxy (orchestrator-core; workspace-persistence: default project).
- [ ] 5.3 Extend the host `EventSink` event set with the surface byte and status events and emit them from the runtime (orchestrator-core: output and status delivered as events; ADR-0022).
- [ ] 5.4 Write test: creating a terminal surface without an explicit project places its session under the seeded Unfiled project (orchestrator-core: default project when none given).

## 6. Desktop host transport

- [ ] 6.1 Bind each surface's byte stream to a per-surface streaming `ipc::Channel` (ordered, high-throughput; not the event system) and bind status changes to the event channel (design: desktop host binds the byte stream to a streaming channel; ADR-0024).
- [ ] 6.2 Bind the create-terminal-surface, input, and resize request methods to host commands (orchestrator-core: embedded in-process by a host).

## 7. SDK terminal-surface client

- [ ] 7.1 Write failing tests: the SDK create returns a `surface_id`; subscribe delivers raw bytes over the surface channel and status; input and resize forward keyed by `surface_id` and never open a daemon connection (sdk-orchestrator-client: typed terminal-surface client).
- [ ] 7.2 Implement the typed terminal-surface client over the orchestrator API and host transport, reading the per-surface byte channel, with wire types centralized in one module (sdk-orchestrator-client; generation deferred to 0.1.4).

## 8. UI terminal pane

- [ ] 8.1 Rewire the terminal pane to accept a `surface_id` and attach through the SDK to the surface byte stream, render raw bytes preserving ANSI, clear and reattach on `surface_id` change, and tear down cleanly on unmount leaving the pseudo-terminal running (ui-terminal-pane: session-scoped terminal connection; terminal output rendering).
- [ ] 8.2 Send a resize for the surface to the orchestrator via the SDK when the container resizes (ui-terminal-pane: resize propagates).
- [ ] 8.3 Drive the connection status indicator and manual reconnect from the surface attachment state (ui-terminal-pane: connection status indicator).

## 9. Retire the engine for the terminal path

- [ ] 9.1 Remove the engine-era WebSocket-to-server terminal transport from the desktop terminal pane and route terminal I/O through the orchestrator over the native transport (desktop-engine-runtime: desktop terminal I/O flows through the surface-runtime; ui-terminal-pane).
- [ ] 9.2 Verify no in-renderer engine carries the terminal pseudo-terminal I/O on the desktop host (desktop-engine-runtime: engine is off the terminal path).

## 10. Verification

- [ ] 10.1 Create a session in the Unfiled project with a terminal surface; the terminal renders and streams live pseudo-terminal bytes through the orchestrator API / `EventSink`, not the engine (acceptance 1).
- [ ] 10.2 Confirm the surface-runtime proxies one pseudo-terminal per surface over `daemon-pty-client`, exposes status, and queues sends (acceptance 2).
- [ ] 10.3 Confirm a surface row persists and, after a host restart, the runtime reconnects to the running daemon by `surface_id` and the live session reattaches (acceptance 3).
- [ ] 10.4 Confirm the engine path for terminal surfaces is off on the desktop host (acceptance 4).
- [ ] 10.5 `turbo test`, `turbo lint`, and `turbo build` pass for the touched packages, and `cargo test` passes for the touched crates (testing gate).
