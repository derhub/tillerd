# 0010. Daemon holds PTY master fds in its main process

- Status: accepted
- Date: 2026-06-02

## Context

Two architectural options were considered for managing multiple concurrent PTY sessions inside the daemon:

1. **Subprocess per session**: spawn a child process per PTY, use socket-based IPC to relay data to the daemon.
2. **In-process fds**: daemon owns all PTY master file descriptors directly in its event loop.

The choice directly affects upgrade semantics: the fd-handoff upgrade path (ADR-0011) requires that the daemon own the master fds, because macOS does not have `/proc`-style fd transfer — the only OS-level mechanism is inheriting open fds across `fork`/`exec`.

## Decision

The daemon owns all PTY master file descriptors directly in its main process. `node-pty` fires async `onData` callbacks via Bun's event loop (liburing/kqueue). No subprocess per session.

Flow control (ADR-0007, D3) bounds per-session work: when all subscribers of a session exhaust their credit the daemon pauses the PTY master fd read at the kernel level, causing the slave-side write to block once the kernel PTY buffer fills. This means no session can starve others by producing unbounded output.

A per-subscriber credit model ensures that one slow client does not pause delivery to other subscribers on the same session. The PTY fd is paused only when **all** subscribers of that session have zero remaining credit.

## Consequences

- PTY master fds are accessible as `PtyTransport._fd` (node-pty private field, pinned to v1.1.0 with a runtime assert).
- The daemon holds N fds for N sessions; there are no extra Bun runtime instances (~20–30 MB each).
- Session isolation is logical (event loop) rather than process-boundary: a native addon crash in node-pty kills the daemon. Accepted for Phase 1.
- The subprocess-per-session approach is ruled out for Phase 1; it would require `SCM_RIGHTS` fd transfer which node-pty does not support.
- The in-process fd layout enables zero-copy fd-handoff upgrade (ADR-0011).
