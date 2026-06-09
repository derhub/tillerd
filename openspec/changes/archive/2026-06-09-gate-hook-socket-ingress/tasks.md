## 1. Shared envelope type

- [x] 1.1 Add the hook-ingestion envelope type to `packages/contracts-rs` (next to `HookSubscribeRequest`): `{ sessionId, token, hook }`, where `hook` is the raw agent payload. Camel-case wire keys, round-trip test.

## 2. gate-notify producer (replaces bin/athing-notify)

- [x] 2.1 Add `apps/gate-notify` crate (Rust binary; depends on `contracts-rs`). Add it to the workspace members.
- [x] 2.2 Implement: read the lifecycle payload from stdin; read `ATHING_DIR`, `ATHING_SESSION_ID`, `ATHING_SESSION_TOKEN` from env; derive `$ATHING_DIR/gate-hook.sock`; wrap stdin in the envelope; write one length-prefixed frame (shared `encode_frame`); exit.
- [x] 2.3 Fire-and-forget: a missing/unreachable socket, a slow write, or any error exits 0 and forwards nothing; bound the runtime. Test the absent-socket and happy paths.
- [x] 2.4 Build the binary to the resolver's expected path (`bin/athing-notify`, or via `ATHING_NOTIFY_BIN`); delete the `bin/athing-notify` bash script. The `platform-bun` resolver (`notifyScriptPath`/`prepareNotifyScript`) is reused unchanged.

## 3. Gate hook face: framed socket

- [x] 3.1 In `apps/gate`, replace the axum HTTP hook route with a `UnixListener` at `$ATHING_DIR/gate-hook.sock` that reads length-prefixed frames via the shared codec.
- [x] 3.2 Decode the envelope and build the same internal `Inbound` (session, token, body) the HTTP path produced; feed it into the existing router/middleware unchanged.
- [x] 3.3 Move auth + normalize to read the token and session from the frame envelope instead of HTTP headers.
- [x] 3.4 Delete `write_gate_url` and the `gate.url` write; remove the hook TCP bind and the hook-face axum app. Confirm the hook face opens no TCP port.
- [x] 3.5 Update gate tests (hook ingress + router integration) to drive the framed socket.

## 4. Adapter install + migration

- [x] 4.1 In `packages/adapter-claude-code/src/setup.ts`, drop the inline gate-mode curl branch; the installed hook command always invokes the resolved notify binary (stdin payload).
- [x] 4.2 Replace the curl-specific idempotency marker (`-A athing-notify`) with a marker the new command carries; update `hasMarker`/install/uninstall detection.
- [x] 4.3 On re-running setup, detect an already-installed curl hook and replace it with the new command.

## 5. Remove gate.url and ATHING_GATE_URL (full reader set)

- [x] 5.1 Remove `write_gate_url`/`gate.url` (done in 3.4) and the `ATHING_GATE_URL` env var; derive `$ATHING_DIR/gate-hook.sock` where the hook target is needed.
- [x] 5.2 Update readers: `apps/desktop/src-tauri/src/orchestrator.rs` (`resolve_gate_url` + env inject), `apps/server/src/index.ts`, `packages/engine/src/daemon/proxy.ts`, `packages/sdk/src/types/session.ts`, `packages/platform-bun/src/{process-launch.ts,setup.ts}`, `apps/memorya/src/dual_mode.rs`.
- [x] 5.3 Update tests referencing `ATHING_GATE_URL` (cli gate-install, engine proxy, platform-bun gate, memorya deployment/recall, contracts correlation-trace).
- [x] 5.4 Remove `gate.url` from `docs/services.md`'s source-of-truth table and adjust the gate entry.

## 6. Remove stale daemon-hook wiring

- [x] 6.1 Remove `HOOKS_SOCK` from `packages/platform-bun/src/supervisor.ts` and its export.
- [x] 6.2 Remove `hooksSocketPath` from `packages/engine/src/engine.ts` (`EngineDeps`) and the `AgentSessionProxy` constructor param/field in `proxy.ts` (it is never read).
- [x] 6.3 Remove the `hookSocketPath: HOOKS_SOCK` argument from the terminal-spawn message in `apps/server/src/index.ts`, and the `HOOKS_SOCK` import.
- [x] 6.4 Remove `ATHING_BRIDGE_URL` handling (the deleted script's bridge branch is gone; remove any remaining injectors/readers) and update/remove `packages/platform-bun/tests/notify-client.test.ts` and the `hooks.sock`/`HOOKS_SOCK` references in `tests/integration/{daemon,engine}.test.ts` and `packages/engine/tests/proxy.test.ts`.

## 7. Verify

- [x] 7.1 `cargo test --workspace` and the TS test suites pass.
- [x] 7.2 End-to-end hook round-trip: `gate-notify` (stdin) → `gate-hook.sock` → gate auth/normalize → fanout to a subscriber.
- [x] 7.3 Confirm the gate hook face binds no TCP port and writes no `gate.url`.
- [x] 7.4 Confirm the MCP face and the admin/subscribe/tool faces are unchanged, and `daemon-pty` is untouched.
