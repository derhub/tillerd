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
- A reproducible `tests/benchmark` harness comparing the reference and Rust daemons on identical
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

### D4: VT model / snapshot production — direct 1:1 port of the reference parser (IMPLEMENTED)

The snapshot frame requires a full grid of cells (char, fg/bg color, attributes) plus cursor.

**Decision (revised during implementation):** the reference `vt-state.ts` hand-rolled parser is
ported **1:1 to Rust** (`packages/daemon-pty/src/vt.rs`) rather than using `alacritty_terminal`. The spec's
hard requirement is snapshot _parity with the reference daemon_; an off-the-shelf VT engine
diverges from the reference's specific erase/tab/wide-char/SGR/default-empty-space semantics and
its closed integer color encoding, so a direct port is the lower-risk path to passing the
golden-fixture parity test — and it drops a churn-prone external dependency (originally flagged as
a risk). Snapshots are built **on demand** from the bounded ring buffer through a fresh parser at
the current dimensions, exactly as the reference does (it keeps no live VT).

- _Alternatives_: `alacritty_terminal` (original plan) — rejected during implementation for the
  parity-divergence and API-churn reasons above. `vte` (parser state machine only) — would re-derive
  the grid/scrollback/SGR state anyway.
- _Parity evidence_: `vt::tests::snapshot_parity_with_reference` feeds a synthetic ANSI stream
  through the Rust parser and asserts cell-for-cell equality against a golden fixture generated by
  the reference `VtState` (`packages/daemon-pty/tests/fixtures/vt-golden.json`).
- _Encoding seam_: `cell.rs` holds the closed integer encoding (default=0, ANSI 1–16, 256-color =
  idx+17, RGB = `0x1000000 | rgb`) and the attribute bitmask, mirroring `snapshot-render.ts`.

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

The Rust daemon is a standalone cargo crate at `packages/daemon-pty/`, built via `cargo build
--release`. It lives under `packages/` for discoverability but carries **no `package.json`**, so
the Bun/turbo workspace globs (which key off `package.json`) never pick it up — it stays out of
the turbo/Bun graph. The `tests/benchmark` runner and any CI job invoke cargo directly; turbo is not
taught about Rust, keeping the Bun-first tooling assumptions intact.

### D10: Benchmark compares the Rust daemon against the Node TS daemon, driving login shells

**Decision (revised during implementation):** the benchmark compares the Rust daemon against the
**Node** build of the TS daemon, not the Bun build. The Bun reference is non-functional for
benchmarking on the current runtime — node-pty's master fd is not a writable process fd under Bun,
so it cannot accept PTY input at all (proven: `dup(fd)` → EBADF). The Node daemon runs the same TS
sources and works, so it is the fair, working baseline. See **D11** (Node-daemon prerequisite).

Both daemons are driven over the real socket protocol with the **login shell** as the session
target (no command → `$SHELL`), the realistic product path: commands are typed into the shell and
echoed back. (An earlier explicit-command variant isolated pure byte-copy overhead; the login-shell
variant is kept because it reflects real usage — note that the shell's own prompt/ZLE work then
dominates the latency tail, which is identical across daemons and so does not distort the
comparison.) The harness (`tests/benchmark/`) selects the binary under test, so any conforming
daemon can be measured.

### D11: Node-daemon prerequisite for a working benchmark baseline (IMPLEMENTED)

Task 11.6 ("run against both the reference and Rust daemons") requires a _working_ reference. The
Bun TS daemon cannot accept PTY input under the current Bun (D10). To produce a fair baseline, the
TS daemon's runtime-specific seams (`Bun.listen` → `node:net`, `Bun.serve` → `node:http`, the
`Bun.file` read loop → node-pty native `onData`) were ported so it runs on **Node**, where node-pty
works natively. `bin/athing-daemon` is now a Node bundle. This is a benchmark enabler for this
change; broadening it into the TS daemon's primary runtime is out of scope here and should land as
its own change.

## Risks / Trade-offs

- **PTY-master fd adoption across daemon upgrade** → _Resolved (post-archive)._ `portable-pty`
  doesn't build a pty from a pre-existing master fd, so adopted sessions bypass it: the inherited
  master fd is owned directly as a raw `File` (read via a shared `Arc<File>`, write to the same fd,
  resize via `TIOCSWINSZ`, EOF = exit, kill by signalling the recorded child pid). Fds are passed by
  **process inheritance** at spawn (mapped to fds `4+i` via `command-fds`), not SCM_RIGHTS — simpler
  and matching the reference daemon's mechanism. `portable-pty` is still used for the spawn path.
  The one `unsafe` boundary (taking ownership of the inherited fd + the resize ioctl) is localized
  behind `#[allow(unsafe_code)]` with SAFETY comments; the crate is otherwise `#![deny(unsafe_code)]`.
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

1. Land the cargo crate and `tests/benchmark` harness; default selection stays the reference daemon.
2. Bring the Rust daemon to green on the new `rust-pty-daemon` conformance scenarios.
3. Run the benchmark to record the comparative baseline.
4. Opt in per-developer/per-deploy by pointing `ATHING_DAEMON_BIN` at the Rust binary.
5. Rollback = revert the env value; the reference daemon resumes with no other change.

## Open Questions (resolved)

- **PTY-master fd reattachment across upgrade** → _Implemented (task 8.2, post-archive)._ Fds are
  passed by process inheritance (mapped to fds `4+i` via `command-fds`) and adopted as raw-`File`
  sessions, bypassing `portable-pty` for the adopt path. Degrade (8.3) is still honored on any
  failure. Verified end-to-end (`tests/benchmark/upgrade-test.ts`).
- **Durable stopped-session store format** → _Shared._ The Rust daemon writes the same file path
  (`~/.athing/stopped-sessions.txt`) in the same newline-delimited format as the reference, so a
  session stopped under one daemon stays stopped after switching to the other. No parallel store.
- **VT crate / scrollback** → _Moot._ D4 now ports the reference parser directly (no
  `alacritty_terminal`); the snapshot is built on demand from the same bounded ring-buffer window as
  the reference, so reconnect fidelity matches by construction.
