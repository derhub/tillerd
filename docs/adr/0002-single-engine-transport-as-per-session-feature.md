# 0002. Single engine; transport is a per-session feature

- Status: accepted
- Date: 2026-06-01

## Context

PTY (raw bidirectional bytes, interactive, resize) and a future headless stream-json transport (typed events, headless, no resize) have fundamentally different shapes. A shared `Transport` interface fitting both would be a leaky lowest-common-denominator. We also want a host/UI to run interactive and headless sessions concurrently without juggling packages.

## Decision

Ship a single `@tillerd/engine`. Transport is selected per session — PTY now, a headless stream-json mode later — implemented as separate internal code paths, not as implementations of a shared `Transport` interface. The two paths converge only on the canonical event model (data/status/content). The engine is created via a factory (`createEngine()`) returning an isolated instance with no module-level mutable state, so one instance hosts many concurrent sessions (mixed modes) and a host may run more than one instance.

## Consequences

- No leaky transport abstraction; each mode is cohesive; one package is simpler to build, version, and run.
- A UI can mix interactive and headless sessions in one engine instance, consumed uniformly via `AgentSession`.
- Adding the second transport is internal work in the engine, not a new package or a core extraction.
- Per-session resource isolation and a single shared loopback receiver routed by session id are required (see ADR-0005).
