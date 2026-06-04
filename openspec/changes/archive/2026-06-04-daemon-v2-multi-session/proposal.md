## Why

The current daemon is a single-process NDJSON relay — adequate for one session, but not designed for concurrent use. With 3-5 simultaneous agent sessions, a stalled PTY blocks all others, unbounded output floods the event loop, and daemon binary updates kill every live session. The daemon needs to be rebuilt to handle concurrent sessions correctly from the start.

## What Changes

- **Wire protocol**: replace NDJSON over Unix socket with length-prefixed binary framing (4-byte header + JSON metadata + optional binary payload). Eliminates JSON-encoding overhead for PTY byte streams and enables clean message boundaries.
- **Protocol handshake**: `hello` / `hello-ack` on connect. Client sends supported protocol versions; daemon picks the highest compatible one and replies with its version. Allows daemon binary to evolve without hard client coupling.
- **Per-PTY subprocess isolation**: each session spawns a child process that owns exactly one PTY. A blocked or high-output PTY cannot stall the daemon event loop or delay other sessions.
- **Flow control**: client sends `ack` frames carrying a consumed-byte count. Daemon tracks per-session credit; when exhausted it pauses the PTY master fd at the kernel level (real backpressure, not a software flag). Resumes on next `ack`.
- **Daemon binary upgrade via fd-handoff**: when a new daemon binary is available the running daemon serialises live sessions to a snapshot file, spawns the successor, passes PTY master fds via stdio inheritance, waits for `upgrade-ack` over an IPC channel, then exits cleanly. Sessions are never interrupted.

## Capabilities

### New Capabilities

- `daemon-wire-protocol`: binary framing format, frame types, protocol version negotiation handshake
- `daemon-pty-subprocess`: per-session PTY subprocess model, IPC between daemon and subprocess
- `daemon-flow-control`: ack-based credit system, kernel-level PTY pause/resume
- `daemon-upgrade`: fd-handoff mechanism, snapshot serialisation, upgrade-ack/nak protocol

### Modified Capabilities

- `pty-daemon`: existing daemon spec gains new session lifecycle model (subprocess-per-session replaces in-process PTY) and new operational states (upgrade-in-progress)
- `session-persistence`: reconnect behaviour is unchanged, but the snapshot format used during upgrade must be compatible with the persistence contract

## Impact

- `packages/daemon/` — full rewrite across server, pty-session, replay-buffer, manifest
- `packages/engine/src/daemon/client.ts` — adapt to new binary framing and handshake
- `packages/engine/src/daemon/proxy.ts` — no API change; internal transport change only
- `packages/engine/src/daemon/supervisor.ts` — add upgrade-trigger path alongside adopt/spawn
- `docs/adr/` — new ADRs for wire protocol choice and subprocess isolation model
