> Prerequisite: `decouple-daemon-from-agent` is applied first; this change ports the resulting
> generic daemon contract to Rust.

## 1. Crate scaffolding and build

- [x] 1.1 Create the `packages/daemon-rs/` cargo crate (binary target producing an `athing-daemon`-compatible binary); kept out of the turbo/Bun workspace graph by carrying no `package.json`
- [x] 1.2 Add dependencies: `portable-pty`, a VT-model crate (pinned exact version), `tokio`, `serde`/`serde_json`, and `nix`/`libc` for raw fd ops
- [x] 1.3 Document the Rust toolchain requirement and `cargo build --release` in a crate README; confirm default Bun/turbo builds are unaffected

## 2. Wire framing and manifest parity

- [x] 2.1 Implement frame encode/decode: 4-byte big-endian length prefix, JSON metadata, optional `0x0a`-separated raw body (mirror `codec.ts`)
- [x] 2.2 Add a cross-implementation round-trip test proving frames encode/decode identically between the reference and Rust framing
- [x] 2.3 Implement the manifest module: atomic tmp+rename write of `{ pid, version }`, removal on clean stop, honoring the base-directory override
- [x] 2.4 Bind the control socket, optional hook ingress socket, and manifest to the same deterministic paths as the reference daemon

## 3. Generic PTY launch and drive plane

- [x] 3.1 Install the login-shell environment at startup so spawned commands inherit a user-terminal environment
- [x] 3.2 Spawn the launch-config command directly (command, args, cwd, env); default to the user's login shell when no command is given; no shell wrapper, no exec-replace
- [x] 3.3 Implement generic resolution: absolute path as-is, bare name via login-shell PATH, default login shell; emit `BinaryNotFound` for an unresolvable named command
- [x] 3.4 Run a per-session blocking reader pumping PTY output into a bounded channel; fan bytes to subscribers and into the VT model with no transformation
- [x] 3.5 Implement raw input write (verbatim, no special-key interpretation) and terminal resize propagation
- [x] 3.6 Implement process teardown with escalating graceful-stop-then-forced-kill, freeing the pseudo-terminal and reporting exit

## 4. Session registry and control channel

- [x] 4.1 Implement the session registry keyed by session id with spawn, kill, and list commands over the IPC channel
- [x] 4.2 Implement additive capability negotiation per connection; missing optional capability degrades to legacy behavior, never rejects
- [x] 4.3 Support multiple concurrent engine clients served independently
- [x] 4.4 Accept re-registration of a recently evicted session id for crash recovery

## 5. Replay buffer and snapshot production

- [x] 5.1 Implement the bounded per-session ring buffer (evict oldest, never unbounded); use it as reconnect payload for non-snapshot-capable clients
- [x] 5.2 Feed PTY output into the VT model and build snapshot frames `{ type, sessionId, rows, cols, cells, cursor }`
- [x] 5.3 Implement the cell-encoding mapping adapter: closed integer color scheme (default, ANSI standard/bright, 256-color offset, 24-bit RGB) and attribute bitmask
- [x] 5.4 Implement wide-character continuation (`""`) and erased-cell (`" "`) conventions and zero-based cursor `{ x, y }`
- [x] 5.5 Emit a snapshot frame before any data events to snapshot-capable subscribers; emit ring-buffer replay to non-capable subscribers
- [x] 5.6 Add golden-fixture tests asserting snapshot parity with the reference daemon for the same input byte streams

## 6. Optional hook ingress

- [x] 6.1 Serve the loopback hook receiver on the stable hook ingress socket only for connections that negotiate the hook capability
- [x] 6.2 Relay authenticated raw hook payloads over the IPC channel to subscribed clients without parsing them
- [x] 6.3 Serve plain terminal sessions with no hook plane, never rejecting on its absence

## 7. Exit qualifier and durable state

- [x] 7.1 Implement exit-qualifier translation with the contracted precedence; attach raw code/signal as diagnostic only
- [x] 7.2 Add table-driven tests mirroring the reference qualifier cases (self-exit ok/error, signal categories, requested stop, external kill)
- [x] 7.3 Implement the durable stopped-session store: persist stopped ids, reject resume with `SessionStopped`, never evict, survive restart
- [x] 7.4 Decide and implement whether the stopped-session store file/format is shared with the reference daemon (resolve the open question)

## 8. Lifecycle: shutdown and upgrade handoff

- [x] 8.1 Implement graceful shutdown on SIGTERM: cascade-terminate all sessions with escalation, emit exit events, exit with no orphaned children
- [x] 8.2 Implement upgrade handoff — adopt running PTY master fds from the outgoing daemon so live sessions survive; clients reconnect and renegotiate. **DONE** (post-archive): the outgoing daemon writes a per-session snapshot, spawns a fresh daemon (`--handoff`) passing the live master fds inherited at fds `4+i` (via `command-fds`), and exits without killing sessions once the successor binds the socket + claims the manifest. The successor adopts each inherited fd into an `Adopted` session (raw-fd read/write, EOF = exit, signal-by-pid). End-to-end verified: a `cat` session keeps echoing input sent _after_ the swap (`tests/benchmark/upgrade-test.ts`).
- [x] 8.3 If fd reattachment proves infeasible in the timebox, gate the handoff behind capability negotiation (degrade, not reject) and record the follow-up — _still honored: any failure in 8.2 (no fd, snapshot/spawn error, successor timeout) leaves sessions on the running daemon (degrade, not reject)._

## 9. Daemon selection

- [x] 9.1 Verify the engine's existing daemon-binary resolution can point at the Rust binary with no engine code or protocol change
- [x] 9.2 Verify selection is reversible — reverting returns the system to the reference daemon with no other change

## 10. Conformance verification

- [x] 10.1 Run the generic daemon and drive-plane contract scenarios against the Rust daemon over its real control socket; all pass
- [x] 10.2 Assert wire interchangeability: engine adopts, negotiates, spawns, subscribes, and drives sessions without distinguishing it from the reference daemon

## 11. Benchmark harness (`tests/benchmark`)

- [x] 11.1 Create the `tests/benchmark` runner (Bun-driven) that launches a selected daemon binary and speaks the real control socket protocol to it
- [x] 11.2 Drive both daemons with the same explicit command (e.g. a flood binary), isolating daemon overhead from launch differences
- [x] 11.3 Implement the fixed workloads: spawn storm, sustained high-throughput output, many concurrent sessions, subscribe/snapshot latency, reconnect replay
- [x] 11.4 Sample resident memory, byte-copy throughput, snapshot build time, and latency percentiles (p50/p95/p99) at the socket boundary
- [x] 11.5 Emit a single comparative report presenting each metric per workload, side by side per daemon binary, attributed to workload and binary
- [x] 11.6 Run the harness against both the reference and Rust daemons to record the comparative baseline
