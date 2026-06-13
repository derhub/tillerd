# Deferred — do not resume planning yet

Parked 2026-06-11. Maps to roadmap 0.1.5; blocked behind the 0.0.x Rust inversion.

## Why parked

The proposal was authored 2026-06-06, before the Rust-inversion ADRs (0020–0023). It
drifted against the current architecture:

- References a `daemon-pty` crate that does not exist. The fd-handoff machinery it tears
  down (`prepareUpgrade` / snapshot / `adoptFromFd` / `--handoff`) currently lives in the
  TS engine (`packages/engine/src/daemon/`), which the 0.0.x inversion retires anyway.
- Names only `rust-pty-daemon` + `session-persistence` as modified, but the primary owner
  of the fd-handoff requirement is the `daemon-upgrade` spec it never lists.
- Frames the supervisor as living in "the orchestrators (desktop/server)"; post-0.0.1 that
  is the single orchestrator crate (ADR-0022).

## Revival condition

After the 0.0.x line lands (Rust orchestrator + daemon as the surface). Re-author the
proposal then: it shrinks to "give the Rust daemon a drain state machine + supersede
ADR-0011," targeting `daemon-upgrade` / `rust-pty-daemon` / `session-persistence`, with no
TS-engine teardown.
