> Prerequisite: `decouple-daemon-from-agent` is applied first; this change ports the resulting
> generic daemon contract to Rust.

## 1. Crate scaffolding and build

- [ ] 1.1 Create the `daemon-rs/` cargo crate (binary target producing an `athing-daemon`-compatible binary), outside the turbo/Bun workspace graph
- [ ] 1.2 Add dependencies: `portable-pty`, a VT-model crate (pinned exact version), `tokio`, `serde`/`serde_json`, and `nix`/`libc` for raw fd ops
- [ ] 1.3 Document the Rust toolchain requirement and `cargo build --release` in a crate README; confirm default Bun/turbo builds are unaffected

## 2. Wire framing and manifest parity

- [ ] 2.1 Implement frame encode/decode: 4-byte big-endian length prefix, JSON metadata, optional `0x0a`-separated raw body (mirror `codec.ts`)
- [ ] 2.2 Add a cross-implementation round-trip test proving frames encode/decode identically between the reference and Rust framing
- [ ] 2.3 Implement the manifest module: atomic tmp+rename write of `{ pid, version }`, removal on clean stop, honoring the base-directory override
- [ ] 2.4 Bind the control socket, optional hook ingress socket, and manifest to the same deterministic paths as the reference daemon

## 3. Generic PTY launch and drive plane

- [ ] 3.1 Install the login-shell environment at startup so spawned commands inherit a user-terminal environment
- [ ] 3.2 Spawn the launch-config command directly (command, args, cwd, env); default to the user's login shell when no command is given; no shell wrapper, no exec-replace
- [ ] 3.3 Implement generic resolution: absolute path as-is, bare name via login-shell PATH, default login shell; emit `BinaryNotFound` for an unresolvable named command
- [ ] 3.4 Run a per-session blocking reader pumping PTY output into a bounded channel; fan bytes to subscribers and into the VT model with no transformation
- [ ] 3.5 Implement raw input write (verbatim, no special-key interpretation) and terminal resize propagation
- [ ] 3.6 Implement process teardown with escalating graceful-stop-then-forced-kill, freeing the pseudo-terminal and reporting exit

## 4. Session registry and control channel

- [ ] 4.1 Implement the session registry keyed by session id with spawn, kill, and list commands over the IPC channel
- [ ] 4.2 Implement additive capability negotiation per connection; missing optional capability degrades to legacy behavior, never rejects
- [ ] 4.3 Support multiple concurrent engine clients served independently
- [ ] 4.4 Accept re-registration of a recently evicted session id for crash recovery

## 5. Replay buffer and snapshot production

- [ ] 5.1 Implement the bounded per-session ring buffer (evict oldest, never unbounded); use it as reconnect payload for non-snapshot-capable clients
- [ ] 5.2 Feed PTY output into the VT model and build snapshot frames `{ type, sessionId, rows, cols, cells, cursor }`
- [ ] 5.3 Implement the cell-encoding mapping adapter: closed integer color scheme (default, ANSI standard/bright, 256-color offset, 24-bit RGB) and attribute bitmask
- [ ] 5.4 Implement wide-character continuation (`""`) and erased-cell (`" "`) conventions and zero-based cursor `{ x, y }`
- [ ] 5.5 Emit a snapshot frame before any data events to snapshot-capable subscribers; emit ring-buffer replay to non-capable subscribers
- [ ] 5.6 Add golden-fixture tests asserting snapshot parity with the reference daemon for the same input byte streams

## 6. Optional hook ingress

- [ ] 6.1 Serve the loopback hook receiver on the stable hook ingress socket only for connections that negotiate the hook capability
- [ ] 6.2 Relay authenticated raw hook payloads over the IPC channel to subscribed clients without parsing them
- [ ] 6.3 Serve plain terminal sessions with no hook plane, never rejecting on its absence

## 7. Exit qualifier and durable state

- [ ] 7.1 Implement exit-qualifier translation with the contracted precedence; attach raw code/signal as diagnostic only
- [ ] 7.2 Add table-driven tests mirroring the reference qualifier cases (self-exit ok/error, signal categories, requested stop, external kill)
- [ ] 7.3 Implement the durable stopped-session store: persist stopped ids, reject resume with `SessionStopped`, never evict, survive restart
- [ ] 7.4 Decide and implement whether the stopped-session store file/format is shared with the reference daemon (resolve the open question)

## 8. Lifecycle: shutdown and upgrade handoff

- [ ] 8.1 Implement graceful shutdown on SIGTERM: cascade-terminate all sessions with escalation, emit exit events, exit with no orphaned children
- [ ] 8.2 Implement upgrade handoff — adopt running PTY master fds from the outgoing daemon via SCM_RIGHTS so live sessions survive; clients reconnect and renegotiate
- [ ] 8.3 If fd reattachment proves infeasible in the timebox, gate the handoff behind capability negotiation (degrade, not reject) and record the follow-up

## 9. Daemon selection

- [ ] 9.1 Verify the engine's existing daemon-binary resolution can point at the Rust binary with no engine code or protocol change
- [ ] 9.2 Verify selection is reversible — reverting returns the system to the reference daemon with no other change

## 10. Conformance verification

- [ ] 10.1 Run the generic daemon and drive-plane contract scenarios against the Rust daemon over its real control socket; all pass
- [ ] 10.2 Assert wire interchangeability: engine adopts, negotiates, spawns, subscribes, and drives sessions without distinguishing it from the reference daemon

## 11. Benchmark harness (`./benchmark`)

- [ ] 11.1 Create the `./benchmark` runner (Bun-driven) that launches a selected daemon binary and speaks the real control socket protocol to it
- [ ] 11.2 Drive both daemons with the same explicit command (e.g. a flood binary), isolating daemon overhead from launch differences
- [ ] 11.3 Implement the fixed workloads: spawn storm, sustained high-throughput output, many concurrent sessions, subscribe/snapshot latency, reconnect replay
- [ ] 11.4 Sample resident memory, byte-copy throughput, snapshot build time, and latency percentiles (p50/p95/p99) at the socket boundary
- [ ] 11.5 Emit a single comparative report presenting each metric per workload, side by side per daemon binary, attributed to workload and binary
- [ ] 11.6 Run the harness against both the reference and Rust daemons to record the comparative baseline
