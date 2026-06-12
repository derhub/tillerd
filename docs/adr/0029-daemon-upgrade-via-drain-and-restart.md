# 0029. Daemon upgrade via drain-and-restart

- Status: accepted, supersedes ADR-0011
- Date: 2026-06-13
- Supersedes: ADR-0011

## Context

ADR-0011 chose zero-downtime upgrades via fd-handoff: the predecessor snapshots live
sessions and forks the successor with PTY master fds inherited across exec. It was the
heaviest daemon subsystem, it forced every PTY fd into one process (ADR-0010), and it
coupled the binary codec and version handshake to the upgrade path — all to deliver a
property a bring-your-own-login, single-user, local-first app rarely needs. The handoff
machinery lived in the retired TS engine; the Rust daemon never implemented it. ADR-0008
already accepts crash = session loss; the planned-upgrade path should be simple and
explicit rather than heroic.

## Decision

Planned daemon upgrades are drain-and-restart, built on the contract's drain phase
(ADR-0028). On a version mismatch the orchestrator's supervision signals drain — the
daemon refuses new sessions and lets active ones finish — waits for the clean exit,
swaps the binary, and starts fresh. There is no auto-kill timer; an explicit
"upgrade now" path terminates active sessions deliberately. Continuity across the
restart comes from workspace persistence plus the agent CLI's own resume. No state is
handed between processes.

## Consequences

- Snapshot serialization, fd inheritance, the upgrade-ack handshake, and handoff codec
  usage are gone; the wire protocol carries client traffic only.
- ADR-0010's in-process-fd mandate is no longer forced by the upgrade path;
  subprocess-per-session becomes an available option (not implemented here).
- **BREAKING**: planned upgrades no longer preserve live sessions — the planned path now
  shares the ADR-0008 posture, mitigated by resume-after-restart.
- The supervisor observes upgrade progress through the manifest lifecycle status, the
  same signal every other consumer uses.
