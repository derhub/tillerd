# 0011. Daemon upgrade via PTY fd handoff

- Status: accepted
- Date: 2026-06-02
- Supersedes: ADR-0008 (Phase 1 scope — "recovery is Phase 2")

## Context

ADR-0008 accepted that daemon crashes lose all live PTY sessions ("the daemon is a new single point of failure; recovery is Phase 2"). This ADR narrows the scope: **planned upgrades** (triggered by a supervisor-detected version mismatch) are now handled without session loss. Unplanned crashes still lose sessions.

Replacing a running daemon binary without restarting it requires transferring the open PTY master file descriptors to the new binary. The only portable OS-level mechanism for this is inheriting file descriptors across `fork`/`exec`: the predecessor daemon spawns the successor with the PTY fds already open in the child process's stdio.

## Decision

### Upgrade protocol

1. **Trigger**: supervisor detects that the on-disk daemon version differs from the running daemon's manifest version. It connects and sends an `upgrade` frame.
2. **Predecessor** calls `prepareUpgrade()`:
   - Serialises live session state to a snapshot file (NDJSON, one record per session: `{ sessionId, pid, cwd, cols, rows, fdIndex, replayBuffer: base64 }`).
   - Snapshot is written atomically (write to `.tmp` then rename).
   - Spawns successor with `stdio: [ignore, inherit, inherit, ipc, ...masterFds]` where `masterFds` are the PTY master fd numbers and fd 3 is an IPC channel.
   - Waits up to 10 s for `{ type: "upgrade-ack" }` from the successor via the IPC channel.
3. **Successor** starts in `--handoff` mode:
   - Reads snapshot; for each record wraps `process.stdio[fdIndex]` as an adopted PTY master fd via `PtyTransport.adoptFromFd`.
   - Binds the daemon socket, starts serving, sends `upgrade-ack` over the IPC channel.
4. **Predecessor** receives ack: updates the manifest with the successor's pid, stops its socket, exits 0.
5. **On failure** (timeout or `upgrade-nak`): predecessor SIGKILLs the successor, logs the reason, and continues serving normally.

### Scope

- Planned upgrades triggered by supervisor version detection only.
- Unplanned crashes (SIGSEGV, OOM, host reboot) still lose all sessions — no crash-recovery. This is unchanged from ADR-0008.
- One concurrent upgrade at a time. The supervisor does not retry automatically on failure.

## Consequences

- Live PTY sessions survive a planned daemon binary upgrade with no interruption from the engine client's perspective.
- The replay buffer accumulated before the upgrade is serialised into the snapshot; the successor continues collecting new data from the inherited fd. Some bytes may be replayed twice across the upgrade boundary — clients are responsible for idempotent rendering.
- `PtyTransport.adoptFromFd` wraps an inherited fd using `node:net.Socket`, bypassing `node-pty` spawn. Resize on adopted sessions uses node-pty's native binding on the raw fd (best-effort).
- The predecessor must not exit before the successor sends `upgrade-ack`. The 10 s timeout ensures the predecessor remains available if the successor crashes during startup.
- The successor must bind the same socket path as the predecessor; the predecessor stops its listener before the successor binds, avoiding EADDRINUSE.
