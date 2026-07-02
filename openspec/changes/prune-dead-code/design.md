# prune-dead-code — design

## Context

The daemon byte-transport predates ADR-0041/0042/0043. Its server half was removed
(`5d990f8e` dropped the websocket twin); the client half survived because it compiles and its
tests pass against themselves. Verified dead by call-site audit: `TauriDaemonTransport`,
`FramedDaemonTransport`, `daemon_connect`, `TauriLogger`, `TauriAppData`, `bindSessionToTerminal`,
and `notification_record` have zero production references outside their own files and tests.
`daemon_session.rs`/`gate_admin.rs` self-document as unwired (`#![allow(dead_code)]`); the
register-before-spawn hook-auth flow they implement belongs to the agent surface, deferred to 1.x
(`terminal-only-0x`). `@tillerd/sdk`'s only live consumers are three type re-exports in
`packages/logger` and a stale `apps/ui` package.json dependency.

## Goals / Non-Goals

**Goals:**

- Zero dead production surface in `apps/ui`, `packages/*`, and `apps/desktop/src-tauri` before the
  architecture freeze.
- `@tillerd/sdk` folded into `@tillerd/logger`; one less workspace package.
- Every removal proven dead by search before deletion; suites stay green after.

**Non-Goals:**

- No guardrail/contract-coverage work (phase 2), no UI pattern migrations (phase 3), no doc/spec
  prose rewrites beyond the two delta specs (phase 4).
- No daemon/gate/wire change; the gate is never purged (`terminal-only-0x`); 0.0.6 frozen
  boundaries (wire protocol, data model, service contract) untouched — the removed TS codec/types
  are a client mirror, not the protocol owner.
- No replacement daemon-lost UX: service health already surfaces daemon death; `DaemonLost` had no
  listener.

## Decisions

- **Delete, don't park.** Dead modules leave the tree; git history is the archive. Applies to
  `daemon_session.rs`/`gate_admin.rs` even though they are future-feature code — 1.x reintroduces
  them against the transport that exists then.
- **Split `transport/tauri.ts` instead of deleting it wholesale**: the `TauriCore`/`TauriChannelLike`
  types and `core.ts` (`isDesktopHost`/`loadTauriCore`) are live; move live types into `core.ts` and
  delete the rest. `transport/index.ts` barrel shrinks to the live exports.
- **Delete both TS packages, no fold.** The planned fold of sdk types into `@tillerd/logger`
  assumed logger was live; a consumer audit found `@tillerd/logger` has zero importers (node-only
  pino logger for the retired TS engine — the webview cannot even load it). No consumer needs the
  `Logger`/`LogContext`/`Resource` types; both packages leave the workspace.
- **`bridge.rs` removal is a straight unwire**: drop the module, its `collect_transport!` entry,
  `BridgeState` management, and the `DaemonLost` specta event. The pty daemon still runs — the
  orchestrator's Rust client (`daemon_pty_api`) is the live path; only the renderer's raw byte
  bridge dies.
- **Comment hygiene rides along**: `command_contract.rs` doc comments that name `@tillerd/sdk`
  switch to naming the generated bindings; no behavior change.

## Risks / Trade-offs

- **Hidden dynamic reference** (string-keyed invoke, test helper import) could break at runtime, not
  compile time → mitigation: rg for every removed symbol and command name after deletion; full
  verify + e2e are the gate.
- **`packages/sdk` deletion touches workspace plumbing** (bun workspaces, turbo graph, tsconfig
  paths) — a missed reference fails the build loudly; acceptable.
- **1.x re-cost**: rewriting `daemon_session`/`gate_admin` later. Accepted at the readiness gate —
  they would need rework against the 1.x transport anyway.
