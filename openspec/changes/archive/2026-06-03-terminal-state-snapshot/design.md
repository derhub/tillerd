## Context

The daemon keeps a bounded ring buffer of raw PTY bytes per session and replays it to subscribing clients. Replay is lossy past the buffer ceiling, and reconstructing screen state from a mid-stream byte offset is incorrect. A parsed terminal-state snapshot fixes both — but it must be introduced without breaking existing clients and without ever killing a live session, because the daemon-survives-restart invariant is absolute.

## Goals / Non-Goals

**Goals:**

- The daemon maintains current terminal state and serves a snapshot to capable clients on subscribe/reconnect.
- The snapshot is transparent above the daemon↔engine boundary (delivered as `data` bytes).
- Introduction is additive and capability-negotiated; no breaking bump, no forced restart.
- Live sessions survive daemon upgrades.

**Non-Goals:**

- Crash recovery / respawn (separate change `session-crash-recovery`).
- Scrollback history in the snapshot (current screen only).
- Exit classification (separate change `exit-qualifier-taxonomy`).

## Decisions

### Decision 1: VT parser runs in the daemon

Parsing must run where raw bytes first arrive — the daemon. This keeps state co-located with the source and lets any number of engine clients receive an authoritative snapshot without re-parsing. **Alternative considered:** parse per-client from the ring buffer — rejected; replay is lossy and a mid-stream start parses incorrectly.

### Decision 2: Snapshot covers the current screen only (no scrollback)

The snapshot encodes the visible cell grid, cursor, and active attributes. This bounds size to `rows × cols × ~10 bytes` (≈19–110 KB for common sizes) and is always complete regardless of session age. **Alternative considered:** include scrollback — rejected; unbounded, and the current screen is what matters for restore.

### Decision 3: Snapshot is a structured IPC frame, converted to escape sequences before the UI

The daemon emits `{ type: "snapshot", sessionId, rows, cols, cells, cursor }` over the daemon↔engine IPC channel. The engine proxy converts the grid to escape sequences (ED2 clear + per-cell cursor positioning + SGR + character writes) and emits raw bytes on the data channel. The server and UI see ordinary `data` bytes — no new frame type at the WebSocket layer, no UI snapshot code. **Alternative considered:** forward the structured frame to the UI — rejected; needs UI-side grid rendering and a new message type across all layers for no benefit.

### Decision 4: Additive, capability-negotiated protocol — never a breaking, restart-forcing bump

The daemon-survives invariant forbids a breaking wire change that would force a daemon restart and kill PTYs. On connect, the engine advertises supported features (e.g. `["snapshot"]`); the daemon records them per connection and serves accordingly: capable -> snapshot, non-capable -> legacy ring-buffer replay. New frames are opt-in; an older engine on a newer daemon keeps working. `VersionUnsupported` is reserved for genuine incompatibility (e.g. agent CLI version per ADR-0007), not missing optional features. The ring buffer is therefore **retained** as the legacy reconnect path.

### Decision 5: Daemon upgrades preserve sessions via successor handoff with PTY adoption

Upgrading the daemon binary SHALL NOT terminate PTYs. The daemon supports successor handoff: a new daemon adopts the running PTY child processes from the outgoing one (the adopted-pid path), re-parenting live sessions instead of killing them. Engines reconnect to the successor and re-run the capability handshake. Because PTYs are separate processes and the wire protocol governs only daemon↔engine IPC, a protocol change rides the handoff transparently. **Alternative considered:** stop/restart the daemon and let engines respawn — rejected; violates the survival invariant and loses in-flight work.

### Decision 6: Resize clears below the overlap — no reflow in v1

Reflowing wrapped lines on resize is hard and error-prone. v1 preserves cells in the overlapping region, clears newly exposed cells, and drops content beyond new bounds — matching common embedded terminals and keeping the snapshot deterministic. **Alternative considered:** full reflow — deferred; high complexity, marginal benefit when only the current screen matters.

## Risks / Trade-offs

- **VT parser correctness** -> A bad parse yields a garbled snapshot. Mitigation: adopt a well-tested headless terminal emulator as the backend; fixture tests of common escape sequences against expected grids. Confirm the parser tracks alternate-screen enter/exit (DECSET 1049) for full-screen programs.
- **Wide-character fidelity** -> Grid->escape-sequence conversion must handle CJK double-width and combining characters or cursor columns drift. Mitigation: explicit Unicode-width tests in the conversion path.
- **Parser CPU overhead** -> Output is parsed synchronously before forwarding. Mitigation: profile; if on the critical path, parse in a microtask after forwarding (snapshot may lag live by one tick).
- **Snapshot/live-stream seam (race)** -> Between snapshot generation and live attach, bytes could be lost or duplicated. Mitigation: capture snapshot and begin the live subscription under the same synchronous tick/lock — snapshot is an exact prefix, live stream the exact suffix.
- **Snapshot latency & backpressure** -> The ~110 KB snapshot write SHALL pass through the existing credit/backpressure path, not bypass it. JSON encode of a 220×50 grid is under 1 ms.
- **Memory per session** -> Grid 80×24 ≈ 19 KB, 220×50 ≈ 110 KB; plus ring buffers. Acceptable for single-user; make grid dims configurable per session.

## Migration Plan

No session-killing steps. The protocol is additive (Decision 4); upgrades preserve sessions (Decision 5).

1. Deploy the new daemon; it adopts running PTYs via successor handoff. Live sessions keep running.
2. The new daemon serves both paths concurrently: snapshot to capable clients, legacy replay to others.
3. Deploy updated engines/servers that advertise `snapshot`; un-upgraded clients silently stay on the legacy path.

**Rollback:** revert the daemon via the same handoff; capable clients fall back to ring-buffer replay automatically. No session loss either way.

## Open Questions

- **Alternate screen:** v1 captures the active buffer only; the chosen parser MUST track alt-screen enter/exit — confirm during parser selection.
- **Resize at the seam:** emit the snapshot at daemon-side authoritative dims; the client resizes after applying it.
