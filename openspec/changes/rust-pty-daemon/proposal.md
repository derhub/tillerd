## Why

The PTY daemon is the lowest, hottest layer in the stack: it owns every PTY, copies raw
bytes from each child process to every subscribed client, maintains a virtual-terminal model
per session to produce reconnect snapshots, and survives engine host restarts. Its wire
contract is already declared language-neutral, so a second implementation can be dropped in
behind the same socket without touching the engine. A native implementation removes the
managed-runtime floor on tail latency and resident memory and gives a fixed, low-overhead
resident process suitable for an always-on background terminal backend that holds many
sessions at once.

The daemon's purpose here is generic: it spawns and drives an interactive **user command** —
by default the user's login shell (`bash`, `zsh`, `fish`, ...), or any configured command.
It is a general terminal backend; an agent CLI is just one of the commands it can run.

## What Changes

> **Prerequisite**: `decouple-daemon-from-agent` lands first. The Rust daemon targets the
> post-decouple generic contract (launch the given command, default login shell; generic
> login-shell PATH resolution; no agent defaults, version gate, exec-replace, or interrupt key).

- Add a standalone PTY daemon implemented in Rust, built on `portable-pty` (the wezterm PTY
  abstraction) for PTY spawn/IO and a native VT parser for snapshot production. It reuses the
  existing daemon wire surface: same Unix sockets (`daemon.sock`, optional `hooks.sock`), same
  manifest (`daemon.json`), same length-prefixed framing, same snapshot cell encoding.
- **Generic launch model**: a session is spawned from a launch config carrying the command,
  arguments, working directory, and environment; when no command is given the daemon launches
  the user's login shell (`$SHELL`). The child IS the target — its shell prompt, echo, and
  live rendering are expected output, not noise to be hidden.
- **Drops the agent-specific launch behavior** from the reference contract: no exec-replace to
  suppress the shell prompt, and no interrupt-key/turn-cancel semantics. The daemon forwards
  raw input bytes verbatim; an interrupt is just whatever bytes the client sends.
- Hook ingress (`hooks.sock`) becomes an **optional capability**, not core: a plain terminal
  session needs no hook plane. It is served only when a client negotiates it.
- The Rust daemon is a **sidecar / interchangeable backend**, not a replacement: the engine
  selects which daemon binary to spawn/adopt. The existing daemon stays in place.
- Add a benchmark harness under `./benchmark` that drives both daemons through identical
  workloads over the real socket protocol and reports comparative resident memory, byte-copy
  throughput, snapshot build time, and latency percentiles.

## Capabilities

### New Capabilities

- `rust-pty-daemon`: A native (Rust) general terminal backend daemon that spawns and drives an
  interactive user command (default: login shell) inside a pseudo-terminal, reusing the
  existing daemon wire surface — detached process + manifest, IPC control channel, session
  registry (spawn/kill/list), bounded per-session replay buffer, snapshot frame production with
  the contracted cell encoding, raw bidirectional byte IO, terminal resize, exit-qualifier
  translation, graceful shutdown, and upgrade handoff. Built on `portable-pty` and a native VT
  parser. Hook ingress is supported as an optional negotiated capability.
- `daemon-benchmark-harness`: A reproducible benchmark suite under `./benchmark` that runs both
  the reference and Rust daemons against identical workloads over the real socket protocol and
  emits a comparative report (memory, throughput, latency percentiles, snapshot cost).

### Modified Capabilities

<!-- None. This change introduces a new, generic terminal-backend capability rather than
     modifying the existing agent-oriented pty-daemon / pty-transport specs. The Rust daemon
     reuses the language-neutral wire contract but deliberately diverges from the agent launch
     semantics; that divergence is defined in the new rust-pty-daemon capability, not by editing
     the existing specs. -->

## Impact

- **New code**: a Rust crate (workspace-external to the Bun/turbo graph) producing an
  `athing-daemon`-compatible binary; a `./benchmark` harness (Bun-driven runner that speaks the
  socket protocol to either binary).
- **New dependencies**: Rust toolchain (cargo) for the daemon crate; `portable-pty` and a
  VT-parsing crate (e.g. `vte` / `alacritty_terminal`) as cargo deps. No new Node/Bun deps in
  the existing packages.
- **Touched, behavior-unchanged**: the engine's daemon-binary resolution must be able to point
  at the Rust binary (path/env selection only; no protocol change).
- **Unchanged**: the wire framing, snapshot cell encoding, the reference daemon, engine client
  logic, and all other packages.
- **Out of scope**: Windows support (v1 is macOS/Linux), retiring the reference daemon, and any
  multi-user/commercial deployment concern.
