# enforce-guardrails — design

## Context

The freeze audit found the guardrail gaps track process, not accident: both recent stream
migrations added commands without contract cases, and the specta/invoke split let untyped commands
accumulate. Phase 1's removals then orphaned the four commands that raw `invoke` existed for.
Closing the gaps and banning the escape hatch makes recurrence structurally impossible.

## Goals / Non-Goals

**Goals:**

- Contract test covers exactly the registered command set (no silent gaps, no dead cases).
- Renderer IPC is typed end to end; raw `invoke` is a CI error outside its two sanctioned homes.
- `infra-only-in-app` enforced at `error`; the last violator fixed.
- One owner for the domain-channel byte-tag convention.

**Non-Goals:**

- No UI data-pattern migrations (phase 3), no doc/ADR-status work (phase 4).
- No new IPC features; the daemon adopt-or-spawn contract at the DAEMON level (pty-daemon spec) is
  untouched — only the desktop's dead duplicate goes.
- No per-frame middleware, no bus changes beyond the test import fix.

## Decisions

- **Delete `supervisor.rs` wholesale.** `daemon_ensure` has zero renderer callers and
  `SupervisorState` is never populated, so `shutdown_owned` on exit is a no-op today — removing it
  changes no behavior. Daemon lifecycle belongs to the embedded orchestrator's supervision
  (ADR-0008: the daemon is deliberately detached and outlives the app).
- **Contract-case parity is asserted, not hoped**: while adding the six cases, keep the existing
  "every registered command answers" enumeration as the source of truth; the six cases follow the
  channel-command pattern already used by `surface_channel` (`__CHANNEL__:<id>` string + req body).
- **The invoke ban is scoped by path**: rule fires on `invoke(` / `.invoke(` in `apps/ui/app/**`
  except `lib/transport/core.ts`, and in `packages/client-bindings/src/**` except
  `tauri_bindings.gen.ts`. The gen file and the core loader are the two sanctioned homes.
- **`bus.rs` test import**: the `#[cfg(test)]` module needs a runtime double; it moves to the
  app-owned edge (the test uses the public app surface or a local fake) rather than naming
  `crate::infra::daemon_pty_api` — exact mechanics decided at the edit against the test's real
  assertions, preserving its observable-behavior contract.
- **Wire-tag dedupe**: expose the existing tag constant/helper from the transport channel sink and
  call it from `spawn_logs_watcher`; the byte layout is declared once.

## Risks / Trade-offs

- **Contract cases for channel commands drive live channel registration** in a mock runtime —
  mitigated by following the proven `surface_channel` case shape.
- **Severity flip could block CI on an unseen violator** — mitigated: `ast-grep scan` run locally
  in the fix-all gate before push.
- **Deleting `store.rs` loses the on-disk `store.json` reader** — acceptable: no code reads or
  writes it; stale files on user disks are inert.
