# 0008. PTY ownership moves to a detached daemon process

- Status: accepted
- Date: 2026-06-02

## Context

ADR-0001 established that the engine drives the agent via an interactive PTY. ADR-0007 requires no orphaned processes and a graceful shutdown contract. In the original design the engine held PTY master file descriptors in-process alongside the application server: a server restart closed those fds, sent SIGHUP to the agent process, and invalidated all hook ingress URLs baked into running sessions.

## Decision

Move PTY master fd ownership into a detached long-lived daemon process (`@athing/daemon`). The engine becomes a client that connects to the daemon over a Unix domain socket. The daemon manages all PTY processes and the hook ingress for the lifetime of the user's session, independent of the application server's process lifetime.

The "no orphans" obligation from ADR-0007 now applies to **daemon shutdown**, not server shutdown. When the server shuts down it unsubscribes from the daemon (releasing event streams) but does NOT kill managed sessions. The daemon retains ownership and applies its own escalating-termination policy when it receives a stop signal or when a session's idle timeout expires.

The hook ingress receiver moves from a random ephemeral port in the engine to a named Unix domain socket at `~/.athing/hooks.sock`. This makes the ingress address stable across server restarts; running sessions do not need reconfiguration.

## Consequences

- PTY sessions survive server process restarts; the server reconnects to the daemon and re-subscribes.
- The daemon is a new single point of failure: if the daemon crashes, all sessions are lost (accepted scope for Phase 1; recovery is Phase 2).
- Engine `shutdown()` semantics change: it unsubscribes from session streams rather than terminating them. A caller that wants to terminate all sessions must call `session.kill()` on each before shutdown.
- `node-pty` moves from `@athing/engine` to `@athing/daemon`; the engine no longer holds any PTY binding.
- The `Engine` interface gains two additive methods: `reconnect` and `listSessions`. Existing callers are unaffected.
