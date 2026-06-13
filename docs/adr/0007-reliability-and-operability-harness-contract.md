# 0007. Reliability and operability harness contract

- Status: accepted
- Date: 2026-06-01

## Context

Wrapping an external interactive CLI for unattended use needs more than a happy-path data flow. Without explicit operability rules the wrapper leaks processes, hangs, corrupts state on duplicate callbacks, or breaks silently when the upstream CLI changes. This mirrors process-supervisor and language-server/debug-adapter host norms: bounded interactions, typed protocol errors, clean teardown, capability/version negotiation.

## Decision

Every engine SHALL honor a reliability contract:

- Graceful shutdown: escalate stop signal -> grace period -> forced kill; capture exit code/signal; emit a terminal exit event; reap and clean up on normal exit and crash (no orphans).
- Timeouts on every external interaction (startup, shutdown grace, idle); a timeout yields a typed error and a defined transition.
- A closed, typed error taxonomy surfaced on the event model (`BinaryNotFound`, `NotAuthenticated`, `SpawnFailed`, `HookInstallFailed`, `TranscriptUnavailable`, `TransportClosed`, `Timeout`, `VersionUnsupported`).
- Authenticated loopback control plane (per-session token verified on every callback).
- Bounded buffering with backpressure (pause/resume) or a logged drop policy; never unbounded.
- Independent plane degradation: a failed status/content plane reports a typed error but does not kill the session.
- CLI version awareness: the adapter declares a supported range and the engine refuses/warns on mismatch.
- Session-correlated structured logs; opt-in raw-I/O capture (off by default, with redaction, because raw I/O can contain secrets).
- Interactive lifecycle handling: skip first-run blockers (trust/onboarding), gate prompt submission on readiness, support interrupt distinct from kill.

## Consequences

- The SDK is fit for unattended/automated use, not just demos.
- More upfront engineering per the contract, but predictable failure modes and testability.
- Raw-I/O capture is a security-sensitive feature and is therefore opt-in and redacted by default.
