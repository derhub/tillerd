## 1. Protocol — SDK

- [ ] 1.1 Add a capability advertisement to the connect handshake (additive): engine sends `{ capabilities: string[] }` (e.g. `["snapshot"]`); daemon records per-connection. Do NOT add a breaking version gate
- [ ] 1.2 Add `snapshot` frame type to the daemon↔engine IPC wire schema (valibot in `@athing/sdk`): `{ type: "snapshot", sessionId, rows, cols, cells: Array<{ char, fg, bg, attrs }>, cursor: { x, y } }`

## 2. VT Parser — Daemon

- [ ] 2.1 Add a headless terminal state parser dependency to `packages/daemon` (VT100/ANSI compatible; must not strip bytes from the PTY output path)
- [ ] 2.2 Implement `VtState` class in `packages/daemon/src/vt-state.ts`: cell grid (`rows × cols`), cursor position, active attributes; exposes `feed(bytes: Uint8Array)` and `snapshot(): SnapshotPayload`
- [ ] 2.3 Fixture-based unit tests for `VtState`: cursor movement, erase sequences (ED/EL), SGR attributes, wrapping, partial writes, CJK double-width and combining characters
- [ ] 2.4 Wire `VtState` into `PtySession`: instantiate on spawn, call `feed()` in the PTY `onData` path before forwarding bytes to subscribers
- [ ] 2.5 Resize `VtState` on `resize`: NO reflow (v1) — preserve cells in the overlapping region, clear newly exposed cells, drop content beyond new bounds
- [ ] 2.6 Confirm the chosen parser tracks alternate-screen enter/exit (DECSET 1049) so the snapshot reflects the active buffer for full-screen programs
- [ ] 2.7 Release `VtState` grid memory on session end (no leak after exit)

## 3. Subscribe / Reconnect — Daemon

- [ ] 3.1 In the subscribe handler, branch on the connection's negotiated capabilities: snapshot-capable → emit a `snapshot` frame (from `vtState.snapshot()`); non-capable → emit legacy ring-buffer replay
- [ ] 3.2 Make snapshot-generation and live-stream attachment atomic w.r.t. the session output path: the snapshot is an exact prefix, the live stream the exact suffix — no byte lost or duplicated at the seam
- [ ] 3.3 Route the snapshot write through the existing credit/backpressure path (do not bypass flow control for the ~110 KB payload)
- [ ] 3.4 Integration test: capable subscriber → first frame is `snapshot` with correct `rows`/`cols`/cursor
- [ ] 3.5 Integration test: non-capable subscriber → receives ring-buffer replay, no `snapshot` frame
- [ ] 3.6 Integration test: output arriving during subscribe appears exactly once across snapshot + live stream (no gap, no overlap)

## 4. Capability Negotiation — Engine

- [ ] 4.1 On daemon connect, advertise supported capabilities (including `snapshot`) in the handshake
- [ ] 4.2 Degrade gracefully when the daemon does not offer a feature: fall back to legacy replay, do NOT reject the connection. Reserve `VersionUnsupported` for genuine incompatibility (e.g. agent CLI version per ADR-0007), not missing optional features
- [ ] 4.3 Unit test: engine against a daemon lacking snapshot support → falls back to ring-buffer replay, connection succeeds
- [ ] 4.4 Integration test: older engine (no `snapshot` advertised) against new daemon → keeps working on legacy path

## 5. Snapshot Rendering — Engine Proxy

- [ ] 5.1 On receiving a `snapshot` frame, convert the cell grid to escape sequences (ED2 clear, then per-cell: CSI H cursor position + SGR attributes + character write) and emit the result as raw bytes on the data channel
- [ ] 5.2 Ensure snapshot bytes are emitted before any subsequent live `data` bytes from the same subscribe event
- [ ] 5.3 Unit test: snapshot frame in → correct escape-sequence bytes out, terminal renders expected screen (incl. wide-char cursor columns)

## 6. apps/server & apps/ui — Transparent Passthrough

- [ ] 6.1 Snapshot bytes forwarded transparently as `data` frames — confirm no special server/ui handling is needed (engine already converts snapshot to bytes)

## 7. Observability & Integration

- [ ] 7.1 Emit session-correlated logs for snapshot generation/emit
- [ ] 7.2 End-to-end test: reconnecting client (live session) receives snapshot bytes and renders correct screen without raw byte replay
- [ ] 7.3 Daemon-upgrade test: upgrade the daemon binary while sessions are live → successor adopts running PTYs, sessions stay alive, engines reconnect and renegotiate — zero session loss
- [ ] 7.4 Mixed-version test: new daemon serves a snapshot-capable engine and a legacy engine concurrently — each gets its negotiated path, neither rejected
