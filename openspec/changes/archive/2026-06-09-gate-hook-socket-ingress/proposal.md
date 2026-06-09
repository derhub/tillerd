## Why

The hook face is the only gate ingress that speaks HTTP over a TCP port. The other three faces (admin, subscribe, tool) are length-prefix frames over Unix sockets at deterministic paths. That one exception forces a whole tail of problems:

- the TCP port is ephemeral, so its address is not derivable and must be published in a bespoke `gate.url` file — written non-atomically and never cleaned up, so it goes stale after a crash;
- `ATHING_GATE_URL` is semantically overloaded — the agent uses it as an HTTP base, while a subscriber consumes it as a socket path;
- the hook wire is hand-rolled in the installed `curl` command, far from the gate's reader — the same producer/consumer drift class we just removed from the frame codec.

Moving the hook face onto the shared framed Unix-socket transport collapses all of this: the gate becomes uniform, the address becomes a derivable path, and `gate.url` plus `ATHING_GATE_URL` disappear.

Context that scopes this change: the daemon hook receiver is **already gone** — ADR-0016 made the daemon pty-only and it advertises no hook face. So the gate is already the sole hook receiver, and the lingering `hooks.sock` / `ATHING_BRIDGE_URL` wiring on the host side (engine, server, the notify client's bridge branch) is **stale dead wiring** pointing at a receiver that no longer exists. This change also removes that dead wiring. The gate must be running for hooks to work — that is a pre-existing property (nothing starts the gate today), not introduced here, and is out of scope.

## What Changes

- **BREAKING** (pre-v1): the agent-facing hook contract changes. The hook face moves from `POST http://<ephemeral>/hook/{session}` (HTTP/TCP, axum) to length-prefix frames over `$ATHING_DIR/gate-hook.sock` (Unix socket), using the shared `contracts::framing` codec already used by the admin/subscribe/tool faces. Auth moves from HTTP headers into the frame envelope. Already-installed hooks must be re-installed.
- Add `gate-notify` — a tiny standalone Rust binary, the canonical hook producer. It reads the hook event JSON on stdin, wraps it in the auth envelope, writes one frame to the socket fire-and-forget, and exits. It is the producer-side peer of `gate-client` (the subscribe consumer), and shares the envelope type with the gate through `contracts-rs`.
- **Replace the existing hook client** `bin/athing-notify` (a bash+curl script) with the `gate-notify` binary at the same resolved location. The host's existing resolver (`platform-bun` `notifyScriptPath`/`prepareNotifyScript`/`ATHING_NOTIFY_BIN`) already resolves and validates an executable at that path, so it is reused unchanged — only the artifact changes from a script to a built binary.
- Add the hook-ingestion envelope type to `contracts-rs` (next to `HookSubscribeRequest`), so producer, server, and any future consumer agree on one definition.
- Collapse the adapter's hook command: today `setup.ts` inlines a gate-mode `curl` one-liner *and* the script carries its own gate-mode `curl` — both go away; the installed command always invokes the resolved `gate-notify` binary. A new idempotency/uninstall marker replaces the curl-specific one (`-A athing-notify`); re-running setup migrates already-installed curl hooks.
- Remove `gate.url` and `ATHING_GATE_URL`: the socket path is derived from `$ATHING_DIR`. All readers are updated (see Impact); the subscriber that already treats the value as a socket path becomes correct.
- Remove the stale daemon-hook wiring: `HOOKS_SOCK`, the engine's `hooksSocketPath` (unused in the proxy), the server's spawn `hookSocketPath`, and `ATHING_BRIDGE_URL`. These point at a daemon hook receiver that no longer exists.
- The MCP face is unchanged — it speaks standard MCP over its own configurable transport, by design, and is not part of the framed-face family.

## Capabilities

### New Capabilities

<!-- None. The new gate-notify binary realizes the existing hook-callback-client
     capability over the new transport; the framed wire is covered by the two
     modified capabilities below. -->

### Modified Capabilities

- `hook-ingress`: the receiver's transport changes from an HTTP loopback listener to a framed Unix-socket listener at a deterministic path, and authentication moves from an HTTP bearer header to a field in the frame envelope. The install requirement changes (the hook command invokes the notify client, not curl).
- `hook-callback-client`: the producer is now a runtime-free native client (`gate-notify`) that frames the lifecycle payload over a Unix socket instead of an HTTP POST, and resolves its endpoint as a derived socket path rather than a published URL. Fire-and-forget, non-blocking delivery is preserved.

## Impact

- **New app:** `apps/gate-notify` (standalone Rust binary; depends on `contracts-rs` for the framing codec + envelope type). Run by the agent, imported by nobody — hence `apps/`. It is built and placed at the resolver's expected location (`bin/athing-notify`, or via `ATHING_NOTIFY_BIN`), replacing the bash script there.
- **Removed file:** `bin/athing-notify` (bash script) — superseded by the binary.
- **`contracts-rs`:** gains the hook-ingestion envelope type.
- **`apps/gate`:** the hook face becomes a `UnixListener` + frame reader that builds the same internal `Inbound`; auth and normalize read the token and session from the frame envelope instead of HTTP headers; the axum hook route and `write_gate_url` are removed. The router/middleware pipeline is otherwise unchanged.
- **`adapter-claude-code` (`setup.ts`):** drop the inline gate-mode curl branch (always use the resolved notify binary); new idempotency marker; migrate already-installed curl hooks.
- **`platform-bun` (`ingress.ts`):** resolver reused as-is; the bridge-mode/`ATHING_BRIDGE_URL` path is removed where present.
- **Removed `gate.url` + `ATHING_GATE_URL` — full reader set updated:** desktop `orchestrator.rs` (`resolve_gate_url` + env inject), `apps/server/src/index.ts`, `packages/engine/src/daemon/proxy.ts`, `packages/sdk/src/types/session.ts`, `packages/platform-bun/src/{process-launch.ts,setup.ts}`, `apps/memorya/src/dual_mode.rs`, and the tests referencing them.
- **Removed stale daemon-hook wiring:** `HOOKS_SOCK` (`platform-bun/supervisor.ts`), `hooksSocketPath` (`engine.ts` `EngineDeps` + `proxy.ts` field), the server spawn `hookSocketPath`, `ATHING_BRIDGE_URL`; update the integration/proxy/notify-client tests that reference them.
- **`docs/services.md`:** drop `gate.url` from the source-of-truth table; update the gate entry.
- **Untouched:** the MCP face, the admin/subscribe/tool faces, `daemon-pty` (already hook-free).
- **Out of scope:** flipping the MCP face to a socket / removing axum entirely; ensuring the gate is started (pre-existing gap); any networked/remote gate transport.
