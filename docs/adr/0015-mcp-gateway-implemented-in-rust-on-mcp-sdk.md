# 0015. MCP gateway is implemented in Rust on the MCP Rust SDK

- Status: accepted
- Date: 2026-06-04
- Supersedes: none

## Context

The project is Bun/TypeScript first, but the PTY daemon is already a native Rust crate
(`packages/daemon-pty`) deliberately outside the Bun/turbo graph, because a detached background daemon
is a delivery artifact, not application code. The gateway (ADR-0013) has the same delivery shape: a
bundled, spawned, long-lived background process inside a desktop app that already ships a Rust
runtime.

## Decision

Implement the gateway as a Rust crate `packages/athing-mcp-gateway-rs`, built on the MCP Rust SDK,
outside the Bun/turbo workspace graph — the same posture as the PTY daemon. The crate keeps a
ports-and-adapters shape mirroring the sdk/engine split: namespacing/routing and the registry are
pure and unit-testable without I/O, while process and network side effects are confined to the
supervisor, backend connections, and transport layer.

The MCP SDK minor version is pinned; the pure core is insulated from SDK API drift, and concrete
signatures are settled at first compile.

## Consequences

- The gateway ships as one small binary with a tiny idle footprint and a single toolchain shared with
  the PTY daemon; the desktop bundle does not gain a second language runtime to run a background
  daemon.
- A Rust toolchain is required to build the crate; default `bun install` / `turbo run` are unaffected,
  as with the PTY daemon.
- The MCP SDK and bidirectional-proxy wiring are more verbose in Rust than in the more mature
  TypeScript SDK; this is accepted because delivery shape is a hard constraint while SDK maturity is a
  convenience.
- The pure core (routing, registry) is portable and testable in isolation, limiting the blast radius
  of SDK changes.
