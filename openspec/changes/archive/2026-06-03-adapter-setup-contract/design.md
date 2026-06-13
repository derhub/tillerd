## Context

The prior change made the adapter import-safe by removing host I/O: hook installation became a
pure plan (`planHookInstall`/`planHookUninstall`) over a single settings value, and a host driver
in the platform package performed the read/backup/write. That keeps the engine-facing module pure,
but it also fixes the adapter's setup to exactly one shape — transform one settings value — and
keeps the orchestration in the host.

A host CLI (written in TypeScript, running on a Bun-class host with full I/O) will drive per-adapter
setup directly. It needs the adapter to own its install/uninstall procedure end to end, not return
a value for the host to apply. Setup is a separate concern from the engine: the engine assumes
setup is already complete and only drives the runtime session.

## Goals / Non-Goals

**Goals:**

- A `defineSetup({ install, uninstall })` contract an adapter implements as two procedures the host
  invokes directly; the adapter owns the full flow.
- The adapter owns the full install/uninstall procedure and the agent-specific decisions, but the
  generic mechanics (read, backup, atomic write) are host capabilities the adapter calls — so they
  are implemented once and reused by every adapter, not duplicated per adapter.
- The adapter stays import-safe (zero host primitives at load and in every contract function): its
  setup procedures touch only the injected capability and pure string operations.

**Non-Goals:**

- The host CLI itself (it consumes this contract; built separately).
- Any enable/disable or "is the adapter set up" status. The engine continues to assume setup is
  complete; reporting or gating on setup state is out of scope.
- Changes to the engine, the loopback hook ingress, the notify script, or the hook payload contract.

## Decisions

### Decision: Setup is two adapter-owned procedures, not a pure plan

Replace `planHookInstall`/`planHookUninstall` (+ the host plan-driver) with a `SetupDefinition`
the adapter builds via `defineSetup({ install, uninstall })`. `install(ctx)` and `uninstall(ctx)`
are `async` procedures that perform the whole flow — read, back up, mutate, write — and the host
invokes them directly. `defineSetup` is a typed identity helper (no I/O) that fixes the shape and
gives a single import surface.

- **Why over the pure plan:** the adapter gains full procedural control (ordering, conditionals,
  more than one target) instead of a single settings-value transform, and the caller is a direct
  function call rather than a host driver that must know read/backup/write ordering. The plan model
  was chosen only to keep the adapter import-safe; a separate entry (below) preserves that without
  the constraint.

### Decision: The host supplies a setup context including a filesystem capability

`install`/`uninstall` receive a `SetupContext` carrying the values the adapter cannot compute on
its own — the resolved notify command, the resolved agent-home, a logger, and a `SetupFs`
capability (`readText`, `writeAtomic`, `backup`, `exists`). The adapter reads the settings via the
capability, computes the next content (full control — which keys, idempotency, ordering), then
calls `writeAtomic`/`backup`. The settings path is the adapter joining its relative path onto the
host-supplied `agentHome` with pure string operations.

- **Why over the adapter owning the mechanics:** backup and atomic write (temp + rename,
  timestamped backup) are generic and easy to get wrong; owning them once in the host means every
  adapter reuses a correct implementation. The adapter still owns the procedure and all
  agent-specific content. The notify command and agent-home are host artifacts, so they are
  injected; the path shape is agent policy, so the adapter assembles it from `agentHome`.

### Decision: Setup is a separate export, not a member of the engine-facing definition

Because the procedures touch only the injected capability and pure strings, setup needs no
I/O-bearing entry — the whole adapter package stays import-safe. Setup is exposed as a separate
`setup` export (a `SetupDefinition`) sibling to the `claudeCode` `AgentDefinition`; the engine-facing
definition carries no setup member.

- **Why:** keeps the engine setup-blind and the contract surfaces cleanly separated — the engine
  imports the definition, the host CLI imports `setup` — while the import-safety guarantee now holds
  for both.

### Decision: Remove the pure-plan API from the contract (breaking, pre-v1)

Drop `hookInstall`, `planHookInstall`, `planHookUninstall` from `AgentDefinition`, the
`HookInstallSpec`/`HookPlan`/`AgentSettings` types from the SDK, and `installHooks`/`uninstallHooks`
from the host package. Pre-v1, contracts break freely; the call sites are the composition root and
a handful of tests, updated in this change. This supersedes the hook-installer shape introduced in
the prior change.

## Risks / Trade-offs

- **Write correctness stays in the host capability** -> the `SetupFs` impl keeps the existing atomic
  write (temp file + rename) and timestamped backup; adapter tests inject a fake `SetupFs` to cover
  install/uninstall/idempotency, and a host test covers the real read/backup/atomic-write ordering.
- **The adapter still depends only on the contract type** -> a test asserts both the engine-facing
  definition and the `setup` export import with no host-primitive access.
- **Breaking removal of the plan API** -> the composition root and tests are updated together so the
  build stays green; no on-disk format change, so existing settings files are unaffected.

## Migration Plan

Single-repo, pre-v1, no runtime migration. Update the SDK contract, the adapter (add the setup
entry, remove the plan functions), the host package (remove the plan-driver), the composition root,
and affected tests together. The on-disk settings/transcript layout is unchanged; existing installs
keep working.

## Open Questions

- None blocking. The host CLI's command surface and packaging are deferred to its own change; this
  change only fixes the contract it will call.
