## Context

The PTY daemon (`@athing/daemon`) owns every PTY master fd, fans raw output bytes to
subscribed clients, maintains a per-session virtual-terminal model for reconnect snapshots,
and survives engine host restarts. Its wire contract — sockets, manifest, framing, snapshot
cell encoding — is explicitly language-neutral.

This change adds a second implementation in Rust, but reframes the daemon's job as a **generic
terminal backend**: it spawns and drives an interactive user command inside a pseudo-terminal,
defaulting to the user's login shell (`$SHELL`). The reference daemon is agent-specific — it
resolves a `claude` binary, version-gates it, launches it via login-shell exec-replace so the
shell prompt never leaks, treats ESC as a turn-cancel, and runs the hook ingress as a core
plane. The Rust daemon deliberately drops those agent semantics: the spawned command is the
target, its prompt and echo are expected output, input bytes are forwarded verbatim, and hook
ingress is an optional negotiated capability.

What the Rust daemon reuses from the contract, drawn from the current implementation:

- **Sockets/paths**: `~/.athing/daemon.sock` (IPC), `~/.athing/hooks.sock` (optional hook
  ingress), `~/.athing/daemon.json` (manifest), honoring `ATHING_DIR`.
- **Framing**: 4-byte big-endian length prefix, then UTF-8 JSON meta; if a body is present, a
  single `0x0a` separator follows the meta and the raw binary body follows that.
- **Manifest**: `{ "pid", "version" }`, atomic tmp + rename, removed on clean stop.
- **Snapshot frame**: `{ type, sessionId, rows, cols, cells, cursor }` with the contracted
  closed integer color encoding and attribute bitmask.
- **Exit qualifier**: one platform-independent qualifier per exit; raw code/signal diagnostic only.
- **Upgrade handoff**: live PTY children adopted by the successor daemon (master fds passed
  across), sessions never terminated to effect an upgrade.

## Goals / Non-Goals

**Goals:**

- A native `athing-daemon`-compatible binary that spawns a configurable interactive command
  (default `$SHELL`) inside a PTY and drives it over the existing wire surface.
- Build on `portable-pty` for PTY spawn/IO and a native VT parser for snapshot production.
- Be drop-in selectable behind the engine's existing daemon-binary resolution (env/path), with
  zero changes to the wire framing or snapshot encoding.
- Generic, agent-agnostic command resolution inside the daemon: explicit path honored; else
  login-shell PATH lookup; else default `$SHELL`. No `claude` default, no hardcoded install
  locations, no CLI version gate.
- A reproducible `./benchmark` harness comparing the reference and Rust daemons on identical
  workloads (resident memory, byte-copy throughput, snapshot build time, latency percentiles).

**Non-Goals:**

- Windows support (v1 is macOS/Linux).
- Retiring or modifying `@athing/daemon` or the agent-oriented `pty-daemon`/`pty-transport` specs.
- Agent-specific behavior in the daemon: turn-cancel/interrupt semantics, agent binary
  defaulting, CLI version gating — these belong to the caller (engine/adapter), not this daemon.
- Changing the wire framing or snapshot cell encoding.
- Multi-user/commercial concerns.

## Decisions

### D1: `portable-pty` for the PTY layer

Use `portable-pty` (wezterm) via `native_pty_system()`. Rationale: ~10x the downloads of the
next Unix PTY crate, maintained as part of a shipping terminal emulator (most-exercised PTY
code in the Rust ecosystem), trait-based runtime selection that mirrors the project's
ports-and-adapters style, and it exposes the master as raw `Read`/`Write` so bytes pass through
untouched (raw-bytes-end-to-end).

- _Alternatives_: `pty` (Unix-only libc wrapper, no maintained abstraction); raw `nix`/`libc`
  `openpty` (reinventing wezterm's hardened layer). Rejected for maturity/scope.

### D2: Generic command launch, default `$SHELL`

A session is spawned from a launch config carrying `command?`, `args`, `cwd`, and `env`. The
spawned process IS the target: no exec-replace to hide a shell, no turn-cancel key. The daemon
forwards raw input verbatim and streams raw output, including the shell's own prompt and echo.

- _Login environment_: the daemon installs login-shell environment (PATH and friends) at
  startup so spawned commands resolve and run as in a user terminal, replicating the reference
  daemon's `shell-env` probe — but it does not exec-replace into a hidden shell per session.

### D3: Generic command resolution inside the daemon

Resolution order: an explicit absolute command path is used as-is; otherwise the command name
is resolved via the login-shell PATH; otherwise, when no command is given, the daemon launches
`$SHELL`. If a named command cannot be resolved, fail with `BinaryNotFound`. The daemon carries
no `claude` default, no hardcoded install locations, and no CLI version gate — those are the
caller's concern.

- _Alternatives_: push all resolution to the caller (daemon spawns only explicit paths).
  Rejected: keeping generic PATH/`$SHELL` resolution in the daemon makes it a usable terminal
  backend on its own while staying agent-agnostic.

### D4: `alacritty_terminal` for the VT model / snapshot production

The snapshot frame requires a full grid of cells (char, fg/bg color, attributes) plus cursor.
Use `alacritty_terminal`'s `Term`/`Grid`, feeding raw PTY output into its parser and reading
the grid to build snapshot cells.

- _Alternatives_: `vte` (parser state machine only) would require hand-building the grid,
  scrollback, wide-char handling, and SGR state — re-deriving what `alacritty_terminal` already
  implements and tests. Rejected: high risk of snapshot divergence from the contract.
- _Adapter boundary_: a mapping function translates cell colors/attrs into the contract's closed
  integer encoding (default=0, ANSI 1–16, 256-color = idx+17, RGB = `0x1000000 | rgb`) and the
  attribute bitmask. This conformance-critical seam gets golden-fixture tests.

### D5: `tokio` runtime, blocking PTY IO offloaded

`portable-pty` master IO is blocking. Run a dedicated blocking reader per session pumping master
output into a bounded channel; the async side fans channel bytes to subscriber connections and
into the VT parser. The Unix-socket control plane (accept loop, per-connection framing, optional
hook ingress) runs on `tokio`.

- _Rationale_: keeps the byte-copy hot path off the executor, gives per-session backpressure via
  the bounded channel (the bounded replay buffer), and isolates a slow subscriber.

### D6: Reproduce framing/manifest/qualifier as pure modules

- Framing: `serde_json` for meta, manual `u32` BE length prefix + `0x0a` body separator,
  matching `codec.ts`. A cross-impl round-trip test proves parity.
- Manifest: atomic tmp + rename, same JSON shape.
- Exit qualifier: port the precedence (`stopped-by-request` > self-exit `ok`/`error` >
  signal-category map > `unknown`) as a pure function with a table-driven test.

### D7: Hook ingress is an optional negotiated capability

A plain terminal session needs no hook plane. The daemon serves the hook ingress socket and
relays authenticated payloads only for connections that negotiate the hook capability; absent
negotiation it degrades (does not reject), consistent with additive capability negotiation.

### D8: Daemon selection is opt-in via the existing resolution path

The engine resolves the daemon binary via env/path (`ATHING_DAEMON_BIN`). Selecting the Rust
daemon is pointing that at the Rust binary — no engine code change, no protocol flag. Rollback is
reverting the env value.

### D9: Cargo crate lives outside the turbo/Bun graph

The Rust daemon is a standalone cargo crate (e.g. `daemon-rs/`), built via `cargo build
--release`. The `./benchmark` runner and any CI job invoke cargo directly; turbo is not taught
about Rust, keeping the Bun-first tooling assumptions intact.

### D10: Benchmark drives both daemons with an explicit command

Because the reference daemon's default launch path is agent-shaped (resolves `claude`, hides the
shell), the benchmark drives BOTH daemons with the same explicit command (e.g. a flood binary
like `cat`/`yes`, or a fixed shell invocation) over the real socket protocol, so the comparison
isolates daemon overhead (framing, fan-out, VT parse) rather than launch differences.

## Risks / Trade-offs

- **PTY-master fd adoption across daemon upgrade** → `portable-pty` does not expose constructing
  a pty pair from a pre-existing master fd, but the contract requires the successor to adopt
  running children. _Mitigation_: handle the raw master fd via `nix`/`libc` for the SCM_RIGHTS
  receive + reattach path, keeping `portable-pty` for the spawn path; if reattach proves
  infeasible in the timebox, gate adoption behind capability negotiation (degrade, not reject)
  and land it as a follow-up. Open question, not assumed solved.
- **Snapshot divergence from the contract** → a native VT model can render cells differently from
  the reference `vt-state`. _Mitigation_: D4's mapping seam is covered by golden fixtures; the
  benchmark's subscribe/snapshot workload asserts frame-shape parity.
- **`alacritty_terminal` API churn** → versioned for alacritty's own use. _Mitigation_: pin the
  exact version, confine usage behind the mapping adapter.
- **Login-environment parity** → spawned commands must see the same PATH/env a user terminal
  gives. _Mitigation_: replicate the reference `shell-env` login-shell probe at daemon startup.
- **Benchmark apples-to-oranges** → the reference daemon's default launch is agent-shaped.
  _Mitigation_: D10 — drive both with the same explicit command.
- **Build-tooling split (cargo vs turbo)** → contributors need a Rust toolchain. _Mitigation_:
  the crate is opt-in; default builds and the reference daemon are unaffected; document the
  toolchain in the crate README.

## Migration Plan

1. Land the cargo crate and `./benchmark` harness; default selection stays the reference daemon.
2. Bring the Rust daemon to green on the new `rust-pty-daemon` conformance scenarios.
3. Run the benchmark to record the comparative baseline.
4. Opt in per-developer/per-deploy by pointing `ATHING_DAEMON_BIN` at the Rust binary.
5. Rollback = revert the env value; the reference daemon resumes with no other change.

## Open Questions

- Is PTY-master fd reattachment feasible with `portable-pty` + `nix`, or must upgrade-handoff be
  staged behind capability negotiation initially?
- Reuse the exact durable stopped-session store file/format the reference daemon uses, or a
  parallel store?
- Exact `alacritty_terminal` version to pin, and whether scrollback depth must match the
  reference ring-buffer bound for reconnect parity.
