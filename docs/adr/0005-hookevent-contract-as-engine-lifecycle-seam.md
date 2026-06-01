# 0005. HookEvent contract as the engine's lifecycle seam

- Status: accepted
- Date: 2026-06-01

## Context

Status comes from the agent's native lifecycle hooks delivered over a loopback HTTP receiver. If the engine's status/content logic were tied to HTTP, tokens, and raw payload shapes, it would be hard to test and impossible to feed from another source. We want a clean boundary between "how lifecycle events arrive" and "what they mean."

## Decision

The engine consumes lifecycle exclusively as a normalized `HookEvent` (`{ sessionId, type, payload? }`) defined in the sdk. A generic ingress component owns the producer side — it binds a single loopback receiver on an ephemeral port, verifies a per-session token, validates the envelope, calls the adapter's `parseHook` to build a `HookEvent`, and routes by session id. The status mapper and content reader are pure consumers of `HookEvent` and know nothing about transport. Hooks are installed once and scoped per session by environment injection (bridge URL + session id + token); dispatch is idempotent.

## Consequences

- Status and content logic are transport-blind and unit-testable by dispatching `HookEvent`s directly.
- Any future producer (unix socket, transcript-derived, stream-json, test stub) drives the engine through the same path with no change to status/content.
- The trust boundary is the producer: whoever can dispatch a `HookEvent` is trusted; the receiver guards it with the per-session token.
- The user's agent settings are mutated once to register the hook; a clean uninstall path is required.
