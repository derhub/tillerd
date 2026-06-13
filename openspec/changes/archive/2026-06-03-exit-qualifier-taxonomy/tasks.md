## 1. SDK — ExitQualifier & Mappings

- [x] 1.1 Add `ExitQualifier` closed string enum to `@athing/sdk`: `ok | error | stopped-by-request | killed | faulted | hangup | interrupted | resource-exceeded | unknown`
- [x] 1.2 Change the exit frame valibot schema so `qualifier: ExitQualifier` is the primary exit field; demote raw platform values to optional `raw: { code?: number, signal?: string, signalName?, signalMeaning?, signalCategory? }`
- [x] 1.3 Add SDK pure mappings: `exitToStatus(q): SessionStatus` (qualifier -> `DONE` | `crashed`) and `isRecoverable(q): boolean`; plus `qualifierToCoarse(q): "user" | "clean" | "unexpected"` as a derived grouping
- [x] 1.4 Add `"crashed"` to the `SessionStatus` union type in `@athing/sdk`
- [x] 1.5 Unit test: `exitToStatus` maps `ok`/`stopped-by-request` -> `DONE` and all crash-class qualifiers -> `crashed`

## 2. SDK — Signal Reference Table

- [x] 2.1 Add `@athing/sdk` signal table: map each standard POSIX signal name -> `{ meaning: string, category: SignalCategory }`; categories: `graceful-termination`, `forced-termination`, `fault`, `job-control`, `resource`, `timer`, `user-defined`, `child`, `window`, `info`
- [x] 2.2 Add platform number->name maps (macOS and Linux) for resolving numeric signals reported by the pseudo-terminal binding; key all lookups by name, never raw number
- [x] 2.3 Add `resolveSignal(signal: string | number): { name: string, meaning: string, category: SignalCategory } | { name: "unknown", raw: string | number }` helper
- [x] 2.4 Add `signalCategoryToQualifier(category, killedByUser): ExitQualifier` (fault->`faulted`, forced-termination->`killed` unless killedByUser, SIGHUP->`hangup`, SIGINT/SIGQUIT->`interrupted`, resource->`resource-exceeded`, else `unknown`)
- [x] 2.5 Unit test: numeric signal resolves to the same name on macOS and Linux maps (e.g. SIGCHLD 17 Linux / 20 macOS -> `SIGCHLD`)
- [x] 2.6 Unit test: signal absent from the table returns the unknown shape with raw value preserved

## 3. Daemon — Exit Qualifier Translation

- [x] 3.1 Add `killedByUser: boolean` flag to `PtySession` state, defaulting to `false`
- [x] 3.2 Set `killedByUser = true` when the daemon's `kill` (or future `stop`) frame handler fires for a session
- [x] 3.3 Translate raw `(code, signal)` -> `ExitQualifier` at exit: `stopped-by-request` if `killedByUser`; else signal-free -> `ok` (code 0) / `error` (non-zero); else `signalCategoryToQualifier(resolveSignal(signal).category, killedByUser)`; else `unknown`
- [x] 3.4 Emit the exit event with `qualifier` primary and raw code/signal + resolved signal name/meaning/category under optional `raw` (diagnostics only)
- [x] 3.5 Confirm the daemon is the ONLY component reading raw platform code/signal for classification
- [x] 3.6 Unit test: code 0, no signal, no kill -> `ok`
- [x] 3.7 Unit test: non-zero code, no signal, no kill -> `error`
- [x] 3.8 Unit test: kill frame then any signal exit -> `stopped-by-request` (intent wins)
- [x] 3.9 Unit test: SIGSEGV without kill -> `faulted`, raw signal name/meaning preserved
- [x] 3.10 Unit test: external SIGKILL without kill frame -> `killed` (distinct from `stopped-by-request`)
- [x] 3.11 Unit test: SIGHUP without kill -> `hangup`; unmapped signal -> `unknown`

## 4. Engine — Crashed Status

- [x] 4.1 On exit frame, compute status via `exitToStatus(qualifier)`; emit `status: "crashed"` before the exit event when it maps to `crashed`, else normal exit flow. Branch only on `qualifier`, never on raw code/signal
- [x] 4.2 Integration test: crash-class qualifier (e.g. `faulted`) -> `crashed` emitted before exit event
- [x] 4.3 Integration test: `stopped-by-request` -> no `crashed` emitted
- [x] 4.4 Integration test: `ok` self-exit -> no `crashed` emitted

## 5. apps/server & apps/ui — Surface

- [x] 5.1 Forward `crashed` status from engine session events to the WebSocket client as `{ type: "status", status: "crashed" }`
- [x] 5.2 Exit/crash indicators render from the qualifier plus optional diagnostic signal meaning (e.g. "faulted — SIGSEGV, program fault"); UI never reads a raw signal number directly
- [x] 5.3 `ok` self-exit shows the existing exit indicator

## 6. Observability (ADR-0007)

- [x] 6.1 Emit session-correlated logs for exit qualifier translation result
- [x] 6.2 Exit-qualifier logs record both the resulting qualifier AND the deciding raw inputs (killedByUser, exit code, signal name/category) so a misclassification is diagnosable from logs alone

## 7. Regression

- [x] 7.1 Regression test: user-initiated kill yields `stopped-by-request`, no `crashed` status, exit event fires normally
- [x] 7.2 Regression test: clean agent self-exit (code 0) yields `ok`, normal exit, no false crash
- [x] 7.3 Cross-layer assertion test: no component above the daemon reads a raw exit code or signal for control flow — qualifier is the only branch input
