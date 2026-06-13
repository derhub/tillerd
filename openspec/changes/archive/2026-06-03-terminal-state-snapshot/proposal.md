## Why

Raw ring-buffer replay on reconnect is lossy: a reconnecting or late-joining client receives truncated history beyond the buffer ceiling and must re-parse raw byte sequences from an arbitrary mid-stream offset to reconstruct the screen. A parsed terminal-state snapshot restores the exact current screen regardless of session age, at bounded size.

## What Changes

- The daemon parses PTY output into an in-memory terminal state (cell grid, cursor, attributes) per session.
- On subscribe/reconnect, a `snapshot`-capable client receives a state snapshot of the current screen; the engine converts it to escape-sequence bytes before the live stream, so the server and UI see ordinary `data` bytes.
- Snapshot delivery is additive and capability-negotiated: clients that do not advertise `snapshot` keep the legacy ring-buffer replay. No breaking protocol bump.
- Daemon upgrades preserve live sessions via successor handoff with PTY adoption.
- Snapshot generation and live-stream attachment are atomic at the seam (no byte lost or duplicated).

## Capabilities

### New Capabilities

- `virtual-terminal-state`: The daemon-side VT parser, the state snapshot on subscribe for capable clients, the atomic snapshot-to-live seam, and the retained ring buffer as the legacy fallback.

### Modified Capabilities

- `pty-daemon`: Subscribe returns a snapshot for capable clients and ring-buffer replay for others; adds additive capability negotiation and session continuity across daemon upgrade.
- `agent-session`: Reconnect delivers a terminal state snapshot before live data.

## Impact

- `@athing/sdk` — `snapshot` IPC frame schema; capability advertisement in the connect handshake
- `packages/daemon` — VT state parser (`vt-state.ts`), parse-on-output wiring, snapshot-on-subscribe, capability negotiation, successor-handoff/adoption preservation
- `packages/engine/src/daemon/proxy.ts` — capability advertisement; convert snapshot frame -> escape-sequence bytes on the data channel
- `apps/server`, `apps/ui` — none beyond receiving snapshot bytes transparently as `data`
- Independent of `exit-qualifier-taxonomy` and `session-crash-recovery`.
