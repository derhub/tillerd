## Context

The gate has four agent-facing ingress faces. Three (admin, subscribe, tool) are length-prefix frames over Unix sockets at deterministic paths. The fourth — hook — is the outlier: HTTP over an ephemeral TCP port, served by axum, with its address published in a bespoke `gate.url` file.

That one exception is the root of a tail of problems already reviewed: a non-atomic, never-cleaned `gate.url`; an `ATHING_GATE_URL` env var overloaded between an HTTP base (agent) and a socket path (subscriber); and a hand-rolled producer wire baked into the installed `curl` command. Notably, the `hook-ingress` spec *already* mandates a named Unix domain socket and forbids an ephemeral TCP port — so the HTTP implementation diverges from its own contract. This change realigns them and brings the hook face into the same shape as the other three.

The hook producer today is the committed `bin/athing-notify` bash script (and a duplicate inline `curl` in the adapter's `setup.ts`). `curl` cannot write a length-prefixed frame, so moving to the framed transport requires a small native client to do the framing — which replaces the script.

The daemon hook receiver is already gone: ADR-0016 made the daemon pty-only (`daemon-pty` advertises no hook face, with tests asserting it). So the `hooks.sock` / `hooksSocketPath` / `ATHING_BRIDGE_URL` wiring still threaded through the engine, server, and the script's bridge branch is **stale dead wiring** pointing at a receiver that no longer exists. The gate is already the sole hook receiver. This change removes that dead wiring rather than removing a live path.

## Goals / Non-Goals

**Goals:**

- The hook face is a framed Unix-socket listener at `$ATHING_DIR/gate-hook.sock`, using the same codec as the other faces.
- A small native producer (`gate-notify`) frames the payload; the agent invokes it instead of `curl`.
- `gate.url` and `ATHING_GATE_URL` are removed; the socket path is derived from `$ATHING_DIR`.
- Fire-and-forget, non-blocking delivery is preserved; per-event latency stays at or below the `curl` baseline.

**Non-Goals:**

- The MCP face is untouched — it speaks standard MCP over its own configurable transport by design, and is not part of the framed-face family.
- Removing axum entirely (the MCP HTTP transport may keep it) is a separate decision.
- Any networked/remote gate transport.

## Decisions

### Framed Unix socket, not HTTP-over-socket

The hook face joins the admin/subscribe/tool pattern: a `UnixListener` at `$ATHING_DIR/gate-hook.sock` whose connections carry length-prefixed frames decoded by the shared `contracts::framing` codec. The decoded envelope is turned into the same internal `Inbound` the HTTP path produced today, so the router and middleware (observe, auth, normalize, fanout) are unchanged downstream. The axum hook route and `write_gate_url` are deleted.

Alternative considered — HTTP over a Unix socket (axum on a `UnixListener`, keep `curl --unix-socket`): rejected. It removes the ephemeral port but keeps HTTP parsing on the hook path and leaves the face inconsistent with the other three. Going fully framed unifies all four faces on one codec and one envelope style.

### `gate-notify` is a small native binary that replaces `bin/athing-notify`

The producer is a new standalone Rust binary, `apps/gate-notify`. It reads the lifecycle payload on stdin, reads the session id, token, and runtime dir from the environment, wraps them in the frame envelope, connects `$ATHING_DIR/gate-hook.sock`, writes one frame, and exits. It depends on `contracts-rs` for the framing codec and the shared envelope type.

It lives in `apps/` (run by the agent, imported by nobody), alongside the other deployable tools, not in `packages/` (libraries others import). It is the producer-side peer of `gate-client` (the subscribe consumer); the symmetry is conceptual, not a directory rule.

It **replaces** the existing `bin/athing-notify` bash script. The host already has a resolver — `platform-bun`'s `notifyScriptPath` (honoring `ATHING_NOTIFY_BIN`, then `bin/athing-notify`) and `prepareNotifyScript` (validates the file exists and is executable). That resolver is reused unchanged: it resolves and validates an executable regardless of whether it is a script or a compiled binary. Only the artifact at that path changes.

Distribution: `gate-notify` is a build artifact, not a committed script. The build places the compiled binary at the resolver's expected location (or `ATHING_NOTIFY_BIN` points at the build output). The committed `bin/athing-notify` script is removed. (How the binary lands at that path in each deployment is an implementation detail of the build/install step.)

Alternative considered — a Bun CLI subcommand: rejected. The hook fires on the agent's hot loop (a tool-use event after every tool call). Bun cold start is ~50–100 ms; `curl` was ~5–10 ms. A native binary starts in ~1–2 ms, so it matches or beats the `curl` baseline. A per-event Bun spawn would be a felt latency regression.

### Remove the stale daemon-hook wiring rather than carry it

Because the daemon no longer receives hooks, the host-side `hooks.sock` plumbing is dead: `HOOKS_SOCK` (`platform-bun/supervisor.ts`), `hooksSocketPath` (an `EngineDeps` field passed to the proxy but never read), the server's spawn `hookSocketPath`, and `ATHING_BRIDGE_URL` (the script's bridge branch + its tests). This change deletes them. The adapter's `setup.ts` also stops inlining a gate-mode `curl` — the installed command is always the resolved notify binary — so the gate-mode curl exists in zero places afterward.

### Single-frame envelope, no handshake

The producer connects, writes exactly one frame, and closes — there is no handshake. One invocation delivers one hook event, so a subscribe-style ready/version handshake would only add round-trips. The envelope mirrors the admin face's in-payload auth:

```json
{ "sessionId": "...", "token": "...", "hook": { /* raw agent payload */ } }
```

The gate's auth reads the token from this envelope (not an HTTP header) and verifies it against the session registry; normalize parses `hook` via the adapter. The envelope type lives in `contracts-rs` next to `HookSubscribeRequest`, so producer, server, and any future consumer share one definition.

### Remove `gate.url` and `ATHING_GATE_URL`; derive the path

With a deterministic socket path, there is nothing to publish. `write_gate_url` and the `gate.url` file are removed, and `ATHING_GATE_URL` is dropped: every consumer derives the specific socket it needs from `$ATHING_DIR`. The subscriber that already treats its gate value as a socket path becomes correct rather than overloaded. Readers updated: the desktop orchestrator's `resolve_gate_url`, the engine's daemon-spawn env injection, the sdk session type, and the subscriber's capture-mode resolution.

### Install and migrate via the adapter

The adapter (`adapter-claude-code/setup.ts`) owns exposure, not the wire. Its installed hook command becomes an invocation of the resolved `gate-notify` binary instead of the `curl` line. The idempotency/uninstall marker — today the `curl` flag `-A athing-notify` — is replaced by a marker the binary command carries (so re-runs detect and uninstall finds it). Re-running setup detects an already-installed `curl` hook and replaces it with the new command.

## Risks / Trade-offs

- **Resolving the `gate-notify` binary path at install time** -> The host must point the hook command at a real, executable path that survives build/distribution. Mitigation: resolve it the same way the current scripted client is resolved (a single stable location), and surface a typed error if the binary is absent — this is already a `hook-callback-client` requirement.
- **Already-installed `curl` hooks** -> Pre-v1 breaking: a session whose settings still carry the old `curl` command posts to a port that no longer exists. Mitigation: setup migration replaces old hooks; the gate no longer binds a TCP port, so a stale `curl` hook simply fails closed (fire-and-forget swallows the error).
- **Stdin payload assumption** -> The producer relies on the agent passing the hook JSON on stdin (the `curl` line already used `--data-binary @-`). If an agent delivered the payload differently, the producer would need another input path. Mitigation: this matches the existing contract; documented as the input.

## Migration Plan

1. Add the hook-ingestion envelope type to `contracts-rs`.
2. Add `apps/gate-notify`: stdin -> envelope -> one frame -> `$ATHING_DIR/gate-hook.sock`, fire-and-forget. Build it to the resolver's expected path; remove the `bin/athing-notify` script.
3. Rewrite the gate hook face: `UnixListener` at `gate-hook.sock` + frame reader building the existing `Inbound`; move auth/normalize to read the envelope fields; delete the axum hook route and `write_gate_url`.
4. Update the adapter install (`setup.ts`): drop the inline gate-mode curl, always use the resolved notify binary, new marker, migrate installed curl hooks. The `platform-bun` resolver is reused unchanged.
5. Remove `gate.url` and `ATHING_GATE_URL`; update every reader (orchestrator, server, proxy, sdk session type, platform-bun process-launch/setup, memorya dual_mode) to derive the path from `$ATHING_DIR`.
6. Remove the stale daemon-hook wiring: `HOOKS_SOCK`, `hooksSocketPath` (engine + proxy), the server spawn `hookSocketPath`, `ATHING_BRIDGE_URL`; update the integration/proxy/notify-client tests.
7. Update `docs/services.md` (drop `gate.url`).
8. Verify: workspace + TS tests; an end-to-end hook round-trip (producer -> socket -> gate -> fanout); confirm the hook face opens no TCP port and writes no `gate.url`; confirm `daemon-pty` and the other faces are unchanged.

Rollback is a straight revert; pre-v1, so no compatibility window is owed. A revert restores the HTTP face, `gate.url`, and the bash client together.

## Open Questions

- Exact build/placement step that lands the `gate-notify` binary at the resolver's path across the desktop (Tauri) and server composition roots — both must resolve the same location. Resolve during implementation; the resolver itself (`notifyScriptPath`/`prepareNotifyScript`) is unchanged.
- Ensuring the gate is actually running is a pre-existing gap (nothing starts it today) and is out of scope here, but it is a hard prerequisite for hooks to be delivered at all.
