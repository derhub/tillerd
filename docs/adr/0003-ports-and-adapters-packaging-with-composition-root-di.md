# 0003. Ports-and-adapters packaging with composition-root DI

- Status: accepted
- Date: 2026-06-01

## Context

The goal is "support all agents" over time while keeping the engine agent-agnostic and letting any UI integrate the SDK. We need a boundary that makes adding an agent cheap and keeps implementation out of the contract.

## Decision

Adopt a ports-and-adapters layout with a strict inward dependency direction:

- `@athing/sdk` — ports and types only (`AgentSession`, `AgentDefinition`, `HookEvent`, event model, status enum, option types); zero deps, zero impl.
- `@athing/engine` — the machinery; depends only on the sdk; never imports a specific adapter.
- `@athing/adapter-<agent>` — implements `AgentDefinition`; depends on the sdk.
- `apps/server` — the composition root: imports the engine and injects a concrete adapter; exposes a session over WS + HTTP.
- `apps/ui` — SPA depending on sdk types and the network only.

A new agent is a new adapter package; the engine and apps are unchanged.

## Consequences

- The contract (sdk) is the stable thing future agents build against; the engine stays agent-blind via dependency injection at the root.
- More packages than a monolith, but the multi-agent seam is explicit rather than implicit.
- Apps are the delivery mechanism and the architecture's first end-to-end test.
