## Why

All session exits are currently reported as a bare platform `(code, signal)` pair, propagated unchanged up through engine, server, and UI. This leaks POSIX semantics into every layer, forces each consumer to re-implement crash detection, breaks when a platform renumbers a signal, and cannot distinguish a clean self-exit from a crash — so a normal agent exit is indistinguishable from a fault.

## What Changes

- Introduce a closed, platform-independent `ExitQualifier` enum in the SDK; the daemon translates raw `(code, signal)` into a qualifier once, at its boundary.
- Every consumer above the daemon branches only on the qualifier. Raw platform values are demoted to optional diagnostic data.
- Add a signal reference table (name/meaning/category) with platform number→name resolution, used internally by the daemon's translation step and for display.
- Add a `crashed` value to the session status contract; the engine emits it when, and only when, the qualifier maps to a crash.
- A single shared mapping (`exitToStatus`) is the sole determinant of crash-vs-clean.

## Capabilities

### New Capabilities

- `exit-classification`: The closed `ExitQualifier` taxonomy, the signal reference table, the daemon-boundary translation, and the qualifier→status mapping that produces `crashed`.

### Modified Capabilities

- `pty-daemon`: Exit events gain a platform-independent qualifier as the primary exit field; raw code/signal become optional diagnostics.
- `agent-session`: Session status gains the `crashed` value, emitted from crash-class qualifiers only.

## Impact

- `@athing/sdk` — `ExitQualifier` enum, `exitToStatus`/`isRecoverable` mappings, signal reference table, platform number→name maps, `crashed` `SessionStatus`, exit frame schema change
- `packages/daemon/src/pty-transport.ts`, `pty-session.ts`, `server.ts` — qualifier translation at exit; raw values demoted to diagnostics
- `packages/engine/src/daemon/proxy.ts` — emit `crashed` via `exitToStatus`, never from raw values
- `apps/server`, `apps/ui` — forward/display `crashed`; render qualifier + diagnostic signal meaning
- This change is a prerequisite for `session-crash-recovery`.
