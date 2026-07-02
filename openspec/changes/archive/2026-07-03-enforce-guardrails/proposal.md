# enforce-guardrails

## Why

Phase 1 removed the dead subsystems; this change makes the frozen architecture self-enforcing and
sweeps the command surface the removals orphaned. Today three guardrails have holes: the desktop
command-contract test silently misses the six commands added by the last two stream migrations, the
renderer can still reach IPC through raw string `invoke` (no compile-time contract), and the
ADR-0038 layer rule `infra-only-in-app` is stuck at `warning` because of one test-only import.
Separately, four registered commands (`log_forward`, `pref_get`/`pref_set` + `registry_*`,
`daemon_ensure`) lost their last callers in phase 1 — they are the renderer-driven daemon/app-data
path from the retired TS engine, superseded by the embedded orchestrator.

## What Changes

- Delete the orphaned desktop command surface and its homes: `diag.rs` (`log_forward`), `store.rs`
  (`StoreState`, `pref_*`, `registry_*`), `supervisor.rs` (`daemon_ensure`, never-populated
  `SupervisorState`, no-op `shutdown_owned`); unwire from `lib.rs`, the transport macro, specta,
  and the contract test.
- Add the six missing contract-test cases: `log_list`, `log_tail`, `logs_changed_channel`,
  `logs_changed_channel_close`, `notification_channel`, `notification_channel_close`.
- Migrate `apps/ui/app/lib/windows.ts` from raw `core.invoke` strings to the generated
  `windowOpen`/`windowFocus`/`windowClose` bindings.
- Add an ast-grep rule banning raw `invoke(` outside `@tillerd/client-bindings` and
  `transport/core.ts`, with rule tests.
- Fix the test-only `crate::infra` import in `shared/bus.rs`; flip `infra-only-in-app` from
  `warning` to `error` (ADR-0038's stated exit condition).
- Deduplicate the domain-channel `0x00` wire tag: `orchestrator_host.rs`'s logs watcher uses the
  tagging owned by the channel sink instead of hand-building the byte prefix.
- `profile_create`: adopt `transport_create!` or document why it cannot (parity with the other
  create commands).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `desktop-app-data`: capability retired — the preference store and session registry have no
  consumers; the orchestrator store owns session data.
- `generated-ipc-bindings`: renderer IPC calls SHALL go through the generated bindings only,
  enforced by a structural rule.

## Impact

- `apps/desktop/src-tauri`: three modules deleted, contract test gains six cases and loses four,
  specta/macro wiring shrinks, logs watcher re-tags via the sink helper.
- `apps/ui`: `windows.ts` on typed bindings; no raw `invoke` remains.
- `.ast-grep/`: one new rule + tests; `infra-only-in-app` severity flip.
- `crates/orchestrator`: `shared/bus.rs` test loses its direct infra import.
- Breaking pre-v1: the four dead commands leave the IPC surface (zero callers).
