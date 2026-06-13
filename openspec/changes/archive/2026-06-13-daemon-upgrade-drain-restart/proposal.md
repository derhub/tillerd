## Why

The PTY daemon supports zero-downtime binary upgrades via file-descriptor handoff (ADR-0011):
on a version mismatch the predecessor serializes live session state, forks the successor with
the PTY master fds inherited across `exec`, waits for an IPC ack, then exits — live sessions
survive the swap. This is well-built, but it is the single heaviest subsystem in the daemon and
it exists to deliver a property a single local user rarely needs: upgrading the daemon binary
without interrupting an in-flight agent run.

The cost is structural and cascading. Fd-handoff forces the daemon to hold every PTY master fd
in one process (ADR-0010) — the only portable way to inherit fds is across `fork`/`exec`, so the
subprocess-per-session option was ruled out, which means one native `node-pty` crash takes down
all sessions. It also couples the wire protocol to the upgrade path: the binary codec (ADR-0009)
is shared between the client wire and the predecessor↔successor channel, and the version
handshake carries upgrade-handoff concerns. Snapshot serialization, `--handoff` mode,
`adoptFromFd`, and double-replay tolerance all exist solely to preserve fds across a planned
upgrade.

For bring-your-own-login, single-user, local-first scope, the bespoke-fit answer is
drain-and-restart: stop accepting new work, let active sessions finish (or resume them via the
agent CLI's own resume + the existing transcript), swap the binary, start fresh. Planned
upgrades already share the failure posture of unplanned crashes in spirit (ADR-0008 accepts
crash = session loss); this change makes the planned path simple and explicit instead of
heroic. Cutting it dissolves ADR-0010's in-process-fd mandate and simplifies ADR-0009.

## What Changes

- **Replace fd-handoff upgrade with drain-and-restart. BREAKING (supersedes ADR-0011).** On a
  supervisor-detected version mismatch, the daemon enters draining: it refuses new sessions and
  checkpoints session metadata, but does NOT kill active interactive sessions on a timer — an
  interactive agent PTY has no natural completion, so a grace-timer would kill a session the user
  is actively using. Drain completes one of two ways: (a) all sessions become idle/closed
  naturally, or (b) the user explicitly chooses "upgrade now," accepting that active sessions are
  terminated and resumed on the new binary. Only then does the daemon remove its manifest and
  exit; the supervisor starts the new binary fresh.
- **Remove the handoff machinery. BREAKING.** Delete `prepareUpgrade`/snapshot-for-handoff, the
  `--handoff` start mode, `adoptFromFd`, the predecessor->successor fd inheritance, and the
  IPC-channel ack. Remove double-replay tolerance tied to the handoff boundary.
- **Relax ADR-0010: in-process PTY fds become an implementation choice, not a mandate.** The
  mandate existed only because fd inheritance required it. With handoff gone, the daemon may keep
  in-process fds (simplest today) or later adopt subprocess-per-session for crash isolation. This
  change does not implement subprocess-per-session; it only removes the constraint that forbade
  it.
- **Narrow ADR-0009 to the client wire only.** Keep length-prefixed binary frames (raw PTY bytes
  need them) AND keep the `hello`/`hello-ack` version handshake unchanged — it gates the _client_
  connection and enables independent client/daemon evolution, which is unrelated to handoff. The
  only removal is the codec's _reuse on the predecessor↔successor IPC channel_, because that
  channel ceases to exist. No change to client-facing framing or negotiation.
- **Continuity becomes "resume," not "survive."** Live PTY sessions do not cross the upgrade.
  Re-attachment after restart relies on the existing session-persistence + the agent CLI's own
  resume and on-disk transcript (ADR-0006), not on preserving the master fd. A session resumed
  after upgrade is a new PTY for the same logical agent session.
- **Upgrade is user-paced.** Because drain lets active sessions finish, an upgrade need not be
  abrupt: the supervisor can offer "upgrade now (restart active sessions)" vs "upgrade when idle."

## Before / After

### Before — fd-handoff (ADR-0011)

```
supervisor sees version mismatch -> sends `upgrade` frame
  predecessor.prepareUpgrade():
    serialize sessions -> snapshot.tmp -> rename
    fork successor: stdio = [ignore, inherit, inherit, ipc, ...masterFds]
    wait <=10s for upgrade-ack over ipc
  successor --handoff:
    read snapshot; adoptFromFd(process.stdio[fdIndex]) per session
    bind socket; serve; send upgrade-ack
  predecessor: update manifest pid; stop socket; exit 0
  (bytes may replay twice across the boundary; clients must dedupe)

requires: in-process fds (0010) + shared upgrade codec (0009) + adoptFromFd + ipc + snapshot
```

### After — drain-and-restart

```
supervisor sees version mismatch -> signals daemon to drain
  daemon:
    stop accepting new sessions (manifest marked draining)
    checkpoint session metadata (cwd, sessionId) for resume
    wait for sessions to go idle/closed naturally
      OR user picks "upgrade now" -> escalate-terminate active sessions (0007)
    (no auto-kill timer: interactive sessions never "finish" on their own)
    remove manifest; exit 0
  supervisor: start new binary (fresh)
  client: re-attach; resumed sessions re-spawn via agent CLI resume + transcript (0006)

requires: drain state + grace timer + the persistence/resume that already exists
removed: snapshot-for-handoff, --handoff, adoptFromFd, ipc handoff, fd inheritance, double-replay
```

## Pros & Cons

### Pros

- Deletes the heaviest daemon subsystem; large net reduction in code and failure modes.
- Un-mandates in-process fds (ADR-0010), unblocking subprocess-per-session crash isolation as a
  future option.
- Simplifies the wire protocol (ADR-0009): no shared upgrade codec, no handoff handshake.
- Upgrade failure modes shrink to "process start/stop," which the reliability contract
  (ADR-0007) already covers — no bespoke handoff-failure handling.
- Honest about the existing posture: planned upgrades now match the already-accepted crash
  posture, instead of a special heroic path that only covers planned upgrades.

### Cons / Costs

- Live sessions do not survive a planned upgrade. A mid-run agent session restarts or resumes
  rather than continuing on the same PTY. Accepted for single-user local scope; softened by
  user-paced drain and CLI resume.
- Resume fidelity depends on the agent CLI's resume + transcript, which is coarser than fd
  preservation (a fresh PTY, replayed context). For interactive TUIs the visual scrollback is
  reconstructed from the replay/transcript, not the original fd stream.
- Removing a shipped capability (fd-handoff is implemented) is a deliberate teardown, not just
  new code. Pre-v1 this is acceptable.

## Scope

### In scope

- Daemon drain state machine: refuse-new, wait-for-idle, explicit "upgrade now" terminate path
  (no auto-kill timer), manifest removal, clean exit (reusing ADR-0007 shutdown semantics).
- Removal of fd-handoff: snapshot-for-handoff, `--handoff` mode, `adoptFromFd`, fd inheritance,
  IPC ack channel, double-replay handling.
- Supervisor change: trigger drain on version mismatch and start the new binary, replacing the
  `upgrade`-frame handoff orchestration.
- Wire-protocol cleanup: keep client framing + client/daemon `hello`; drop handoff codec usage.
- Resume-after-restart wiring through existing session-persistence + transcript-based recovery.

### Out of scope

- Implementing subprocess-per-session (this change only removes the constraint forbidding it).
- Crash recovery for unplanned crashes (SIGSEGV/OOM/reboot) — unchanged; still loses sessions.
- The composability/dedup work (separate change `composable-standalone-tools-architecture`).
- Gateway lifecycle (ADR-0013) — the gateway carries no fds and has no handoff path.
- Windows support.

## Capabilities

### New Capabilities

<!-- none — upgrade behavior lives in the existing rust-pty-daemon capability -->

### Modified Capabilities

- `rust-pty-daemon`: the upgrade requirement changes from fd-handoff continuity to
  drain-and-restart; the in-process-fd requirement is relaxed to an implementation choice. The
  "Wire-compatible reuse of the daemon protocol surface" requirement is amended — the
  predecessor↔successor handoff channel is removed from the contracted surface (client framing and
  `hello` are unchanged), and the spec intro's "upgrade handoff" clause is replaced with
  drain-and-restart. The `hello`/`hello-ack` client handshake is explicitly retained.
- `session-persistence`: the "Reconnect survives daemon upgrade" requirement (and its scenarios
  "Session id stable across upgrade" and "Replay buffer available after upgrade") is rewritten to
  resume-semantics. After a planned upgrade a pre-upgrade session id is resolvable only once
  re-spawned via resume; the replay buffer is reconstructed from the persisted metadata +
  transcript, not preserved continuously across the swap. "Survive the swap" becomes "resume after
  restart."

## Impact

- **Crates:** `daemon-pty` loses the handoff path (snapshot-for-handoff, `--handoff`, adopt-from-fd,
  IPC ack) and gains a drain state machine; the codec module sheds handoff usage. Supervisor code
  in the orchestrators (desktop/server) replaces upgrade-frame orchestration with a drain trigger
  - start.
- **Protocol:** client wire framing and `hello` negotiation stay; the predecessor↔successor
  channel and its frames are removed.
- **ADRs:** supersedes ADR-0011 (fd-handoff); amends ADR-0010 (in-process fds: mandate ->
  implementation choice); simplifies ADR-0009 (codec scoped to client wire). A new ADR records
  drain-and-restart as the upgrade strategy and the supersession/amendment chain. ADR-0008's
  "crash loses sessions" posture is unchanged and now also describes planned upgrades.
- **Data:** the handoff snapshot file format is removed; session-persistence metadata is reused
  for resume.

## Things to Remember

- Breaking changes are acceptable pre-v1; remove the handoff code outright, no compatibility
  shim, no dual-path.
- Drain reuses the ADR-0007 escalating-termination machinery for the explicit "upgrade now" path —
  do not invent a second shutdown path. The new concepts are "refuse new sessions while draining"
  and "wait for idle." Do NOT add a grace-timer that auto-kills interactive sessions.
- Keep the client/daemon `hello` version negotiation — it still guards client/daemon
  compatibility independent of upgrades. Only the handoff-specific handshake goes.
- "Survive" (live fd preserved) is replaced by "resume" (fresh PTY, replayed context). Be precise
  in specs and UI copy — they are different guarantees.
- Relaxing ADR-0010 is a constraint removal, not a rewrite. Do not implement subprocess-per-session
  in this change; just stop requiring in-process fds for upgrade reasons.
- This change is independent of and must not entangle the composability change; coordinate only at
  the daemon-lifecycle seam.

## Where to Start

1. Specify the drain state machine in `rust-pty-daemon`: states (serving -> draining -> stopped),
   refuse-new behavior, wait-for-idle, explicit "upgrade now" terminate path, manifest removal.
2. Re-point the supervisor: on version mismatch, signal drain + start the new binary, instead of
   sending the `upgrade` frame and orchestrating handoff.
3. Delete the handoff path in `daemon-pty` (snapshot-for-handoff, `--handoff`, adopt-from-fd, IPC
   ack) and the codec's handoff usage; keep client framing + `hello`.
4. Wire resume-after-restart through session-persistence + transcript recovery; define the
   "resume not survive" guarantee in `session-persistence`.
5. Write the ADR: drain-and-restart supersedes ADR-0011, amends ADR-0010, simplifies ADR-0009.
