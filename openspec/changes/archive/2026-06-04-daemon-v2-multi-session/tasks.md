## 1. Binary Framing Codec

- [x] 1.1 Implement `FrameEncoder` — writes 4-byte big-endian length prefix + JSON metadata + optional newline + binary body
- [x] 1.2 Implement `FrameDecoder` — stateful streaming parser that reassembles frames from arbitrary-size byte chunks
- [x] 1.3 Add round-trip tests: frame with binary body, frame without body, multi-frame stream, incomplete frame across two chunks
- [x] 1.4 Export codec from `packages/daemon/src/protocol/codec.ts`

## 2. Protocol Types and Frame Catalogue

- [x] 2.1 Define TypeScript union types for all client→daemon frame types (`hello`, `spawn`, `kill`, `list`, `subscribe`, `unsubscribe`, `input`, `resize`, `interrupt`, `ack`)
- [x] 2.2 Define TypeScript union types for all daemon→client frame types (`hello-ack`, `spawn-ack`, `list-ack`, `data`, `exit`, `hook`, `error`)
- [x] 2.3 Define valibot schemas for all frame types with typed parse helpers
- [x] 2.4 Remove old `packages/daemon/src/protocol.ts` and `packages/daemon/src/ndjson.ts` once all consumers migrated

## 3. Protocol Handshake

- [x] 3.1 Daemon: on new socket connection, require `hello` frame before processing any other frame; respond with `hello-ack` if version compatible, `error` + close if not
- [x] 3.2 Engine client (`packages/engine/src/daemon/client.ts`): send `hello` with `versions:[1]` immediately after connect; parse `hello-ack` before resolving the connect promise; throw `AtError("TransportClosed")` on `EVERSION` error
- [x] 3.3 Add test: compatible handshake completes, incompatible version closes connection, message before handshake rejected

## 4. Daemon Server — Binary Framing Migration

- [x] 4.1 Replace `NdjsonDecoder` + `encode()` in `server.ts` with `FrameDecoder` + `encodeFrame()` per connection
- [x] 4.2 Replace `ClientMessageSchema` (old op-keyed) with `ClientFrameSchema` (new type-keyed) in message dispatch
- [x] 4.3 Replace all `ev:`-keyed daemon→client writes with `encodeFrame()` calls using new `DaemonFrame` types
- [x] 4.4 Update `subscribe` handler: send replay buffer as individual `data` frames (binary body = raw bytes) instead of JSON chunks array
- [x] 4.5 Update `spawn` handler: send `spawn-ack` with pid instead of `spawned` event

## 5. Flow Control

- [x] 5.1 Add per-subscription credit counter (initial value 65536 bytes) to the subscription record in the daemon
- [x] 5.2 Deduct `bodyLen` from credit before sending each `data` frame to a subscriber; pause PTY master fd read when all subscriber credits are zero
- [x] 5.3 Handle `ack` frame from client: add `bytes` to credit; if was zero and now positive, resume reading PTY fd
- [x] 5.4 Per-subscriber credit: pausing one subscriber SHALL NOT pause delivery to other subscribers on the same session
- [x] 5.5 On subscribe, pre-fill credit to `max(initialWindow, replayBuffer.length + initialWindow)` so replay never stalls mid-delivery
- [x] 5.6 Engine client proxy: send `ack(n)` after each `data` frame is forwarded to the WebSocket client
- [x] 5.7 Add tests: credit exhaustion pauses data, ack resumes, two subscribers independent

## 6. Snapshot Serialisation

- [x] 6.1 Implement `writeSnapshot(path, sessions)` — NDJSON, one record per session: `{ sessionId, pid, cwd, cols, rows, fdIndex, replayBuffer: base64 }`, atomic write (tmp + rename)
- [x] 6.2 Implement `readSnapshot(path)` — parses NDJSON, returns typed session record array
- [x] 6.3 Add `getMasterFd()` to `PtySession`/`PtyTransport`: reaches into node-pty `_fd` with runtime assert `typeof fd === "number"`
- [x] 6.4 Add snapshot round-trip tests: write → read → records match, atomic write leaves no partial file

## 7. Daemon Binary Upgrade — Predecessor

- [x] 7.1 Add `prepareUpgrade()` to `DaemonServer`: collect master fds from all sessions, build stdio array (`[ignore, inherit, inherit, ipc, ...fds]`), write snapshot, spawn successor with `--handoff --snapshot=<path> --socket=<path>`
- [x] 7.2 Wait for `upgrade-ack` from successor via IPC channel (fd 3), 10 s timeout
- [x] 7.3 On ack: update manifest with successor pid, close own socket, exit 0
- [x] 7.4 On timeout or `upgrade-nak`: SIGKILL successor, log reason, continue serving normally
- [x] 7.5 Add test: nak causes predecessor to survive and continue serving

## 8. Daemon Binary Upgrade — Successor

- [x] 8.1 Add `--handoff` mode to `main.ts` (detected via `process.argv.includes("--handoff")`)
- [x] 8.2 `runHandoffReceiver()`: read snapshot + socket path from argv, parse snapshot
- [x] 8.3 For each snapshot record: wrap `process.stdio[fdIndex]` as inherited PTY master fd into a `PtySession` via `adoptFromFd(fd, meta)`
- [x] 8.4 Add `adoptFromFd(fd, meta)` to `PtyTransport`: attaches to an already-open master fd instead of spawning a new PTY
- [x] 8.5 Bind daemon socket, start serving; send `{ type: "upgrade-ack", successorPid: process.pid }` over IPC channel
- [x] 8.6 Add integration test: spawn session, trigger upgrade, verify session output continues after upgrade

## 9. Supervisor Upgrade Trigger

- [x] 9.1 Add `version` field to manifest read in `packages/engine/src/daemon/supervisor.ts`
- [x] 9.2 After adopting running daemon, compare manifest version against `EXPECTED_DAEMON_VERSION`; if mismatch, send upgrade frame
- [x] 9.3 Add `triggerUpgrade(client)`: sends `{ type: "upgrade" }` frame to daemon
- [x] 9.4 Daemon handles `upgrade` frame: calls `prepareUpgrade()`
- [x] 9.5 Add test: supervisor detects version mismatch and triggers upgrade

## 10. Engine Client — Binary Framing Migration

- [x] 10.1 Replace readline NDJSON parser in `packages/engine/src/daemon/client.ts` with `FrameDecoder`
- [x] 10.2 Replace all send calls with `encodeFrame()`
- [x] 10.3 Verify proxy tests still pass (`packages/engine/tests/proxy.test.ts`)

## 11. ADRs

- [x] 11.1 Write `docs/adr/0009-binary-framing-protocol-for-daemon-ipc.md` — D1 + D5 from design
- [x] 11.2 Write `docs/adr/0010-daemon-holds-pty-master-fds.md` — D2 + D3 from design
- [x] 11.3 Write `docs/adr/0011-daemon-upgrade-via-fd-handoff.md` — D4 from design; narrows ADR-0008 Phase 1 scope

## 12. Integration Tests

- [x] 12.1 Multi-session: spawn 3 sessions concurrently, verify all receive output independently
- [x] 12.2 Flow control: exhaust credit on one session, verify other session unaffected; resume, verify delivery
- [x] 12.3 Full upgrade cycle: 2 live sessions, trigger upgrade, verify both survive with output intact
- [x] 12.4 Reconnect after upgrade: disconnect engine client, upgrade daemon, reconnect, verify replay buffer intact
