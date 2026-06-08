# athing-daemon (Rust)

A native, drop-in alternative to `@athing/daemon`: a generic terminal-backend
daemon that spawns and drives an interactive user command (default: the login
shell) inside a pseudo-terminal, reusing the existing daemon wire surface
unchanged — same Unix sockets, manifest, length-prefixed framing, and snapshot
cell encoding.

It is a **sidecar / interchangeable backend**, not a replacement. The engine
selects which daemon binary to spawn; the reference daemon stays in place.

## Toolchain

Requires a Rust toolchain (cargo). This crate lives **outside** the Bun/turbo
workspace graph — default `bun install` / `turbo run` builds are unaffected and
do not require Rust. Build it explicitly:

```sh
cd packages/daemon-pty
cargo build --release        # produces target/release/athing-daemon
cargo test                   # unit + parity tests
```

## Selecting it

The engine resolves the daemon binary via `ATHING_DAEMON_BIN` (then `./bin/athing-daemon`,
PATH, `~/.local/bin`). Point it at the Rust binary — no engine code or protocol
change:

```sh
export ATHING_DAEMON_BIN=$(pwd)/packages/daemon-pty/target/release/athing-daemon
```

Reverting the env value returns the system to the reference daemon with no other
change.

## What it reuses (wire contract)

- **Sockets/paths**: `~/.athing/daemon.sock`, optional `~/.athing/hooks.sock`,
  `~/.athing/daemon.json` (manifest), honoring `ATHING_DIR`.
- **Framing**: 4-byte big-endian length prefix, UTF-8 JSON meta, optional
  `0x0a`-separated raw body.
- **Manifest**: `{ "pid", "version" }`, atomic tmp + rename, removed on clean stop.
- **Snapshot frame**: `{ type, sessionId, rows, cols, cells, cursor }` with the
  contracted closed-integer color encoding and attribute bitmask.
- **Exit qualifier**: one platform-independent qualifier per exit; raw
  code/signal diagnostic only.

## What it deliberately drops (vs the agent-oriented reference)

No exec-replace to hide a shell prompt, no agent binary default, no CLI version
gate, no interrupt-key/turn-cancel semantics. Input bytes are forwarded
verbatim. Hook ingress is an optional negotiated capability, not a core plane.

## Implementation notes

- **PTY**: built on `portable-pty` (wezterm) for spawn/IO; the master is read and
  written as raw bytes (raw bytes end to end).
- **VT model**: the reference `vt-state.ts` parser is ported 1:1 to Rust rather
  than using an off-the-shelf VT crate, so snapshot output matches the reference
  daemon cell-for-cell. Snapshots are built on demand by replaying the bounded
  ring buffer through a fresh parser at the current dimensions.
- **Runtime**: `tokio` for the control plane; a dedicated blocking reader thread
  per session pumps PTY output into the event loop with per-subscriber credit
  flow control.
- **Upgrade handoff**: live PTY-master fd adoption across a binary upgrade
  (SCM_RIGHTS) is not yet enabled; an `upgrade` request degrades (sessions stay
  on the running daemon) rather than rejecting. Tracked as a follow-up.
