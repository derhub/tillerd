## Context

Session exits are reported as raw `(code, signal)` and flow unchanged through every layer. There is no distinction between a clean self-exit (code 0), a non-zero error exit, an engine-initiated kill, and a fault. Consumers cannot reliably detect a crash, and any per-consumer attempt would re-implement platform-specific logic that breaks across OS signal-number differences.

## Goals / Non-Goals

**Goals:**

- A closed, platform-independent `ExitQualifier` is the only exit contract above the daemon.
- The daemon is the single place that reads raw platform exit values for classification.
- One shared mapping determines crash-vs-clean; the `crashed` status derives from it.
- A signal reference table gives every signal a name, meaning, and category, resolved consistently across platforms.

**Non-Goals:**

- Crash recovery / respawn (separate change `session-crash-recovery`).
- Terminal state snapshot (separate change `terminal-state-snapshot`).
- Distinguishing an in-terminal Ctrl+C from a genuine crash (deferred).

## Decisions

### Decision 1: Ternary classification, not binary

A binary "killed-by-user or not" flag misclassifies every normal self-exit as a crash. Classification is ternary at heart — `user`, `clean`, `unexpected` — but is expressed through the richer `ExitQualifier` (Decision 3); the coarse triple is a derived grouping.

| Coarse class | Condition                                         |
| ------------ | ------------------------------------------------- |
| `user`       | a `kill`/`stop` frame was received before exit    |
| `clean`      | no kill/stop, code 0, no signal                   |
| `unexpected` | no kill/stop, non-zero code OR terminating signal |

Exit code is consulted only for the no-kill case; when a kill/stop frame was sent the result is `user` regardless of code or signal.

### Decision 2: Signal reference table — map by NAME, not number

The SDK ships one table mapping each standard POSIX signal to `{ name, meaning, category }`. Categories:

| Category               | Signals                                                   | Meaning                                   |
| ---------------------- | --------------------------------------------------------- | ----------------------------------------- |
| `graceful-termination` | SIGTERM, SIGINT, SIGQUIT, SIGHUP                          | Asked to stop / hangup / Ctrl+C / Ctrl+\\ |
| `forced-termination`   | SIGKILL                                                   | Force-killed; uncatchable                 |
| `fault`                | SIGSEGV, SIGABRT, SIGFPE, SIGBUS, SIGILL, SIGSYS, SIGTRAP | Program fault / crash                     |
| `job-control`          | SIGSTOP, SIGTSTP, SIGCONT, SIGTTIN, SIGTTOU               | Pause / resume / background tty access    |
| `resource`             | SIGPIPE, SIGXCPU, SIGXFSZ                                 | Broken pipe / CPU or file-size limit      |
| `timer`                | SIGALRM, SIGVTALRM, SIGPROF                               | Timer / alarm expiry                      |
| `user-defined`         | SIGUSR1, SIGUSR2                                          | Application-defined                       |
| `child`                | SIGCHLD                                                   | Child process state change                |
| `window`               | SIGWINCH                                                  | Terminal size changed                     |
| `info`                 | SIGURG, SIGINFO, SIGPWR, SIGSTKFLT                        | Platform-specific informational           |

**Platform trap:** signal _numbers_ differ across platforms (SIGCHLD is 17 on Linux, 20 on macOS; SIGUSR1/2, SIGBUS also diverge). The pseudo-terminal binding reports a name on some platforms, a number on others. The table is keyed by **name**; a reported number is resolved through a platform-specific number->name map before lookup. Logic and display never branch on a raw number.

### Decision 3: A closed `ExitQualifier` is the only exit contract downstream

The daemon translates `(code, signal)` — via the signal table and the `killedByUser` flag — into one closed enum, defined once in the SDK. Every consumer above the daemon branches only on it; raw values are demoted to an optional `raw` diagnostic payload.

| Qualifier            | Source condition                                             | Coarse class | Crashed? |
| -------------------- | ------------------------------------------------------------ | ------------ | -------- |
| `ok`                 | self-exit, code 0, no signal                                 | clean        | no       |
| `error`              | self-exit, non-zero code, no signal                          | unexpected   | yes      |
| `stopped-by-request` | engine `kill`/`stop` caused it (any code/signal)             | user         | no       |
| `killed`             | external forced kill (SIGKILL not from the engine, e.g. OOM) | unexpected   | yes      |
| `faulted`            | fault-category signal                                        | unexpected   | yes      |
| `hangup`             | SIGHUP                                                       | unexpected   | yes      |
| `interrupted`        | SIGINT/SIGQUIT not from the engine                           | unexpected   | yes      |
| `resource-exceeded`  | SIGPIPE/SIGXCPU/SIGXFSZ                                      | unexpected   | yes      |
| `unknown`            | anything unmapped                                            | unexpected   | yes      |

Translation precedence: `killedByUser` -> `stopped-by-request`; else no-signal -> `ok`/`error` by code; else the signal category selects the qualifier; else `unknown`.

The SDK exposes pure functions so no consumer re-derives the mapping: `exitToStatus(q): SessionStatus` (-> `DONE` | `crashed`) and `isRecoverable(q): boolean`. The coarse `user`/`clean`/`unexpected` triple is a derived view of the qualifier, not an independently computed field.

**Why closed enum over passing signals through:** consumers stay platform-agnostic and stable across OS differences; crash detection lives in one tested mapping. **Alternative considered:** carry `(code, signal)` to each consumer — rejected; it leaks platform semantics into every layer and breaks on signal renumbering.

## Risks / Trade-offs

- **Misclassification is silent** -> A wrong qualifier produces a wrong status. Mitigation: log the deciding inputs (killedByUser, code, signal name/category) alongside the resulting qualifier so any misclassification is diagnosable from logs alone.
- **Known limitation** -> Ctrl+C pressed _inside_ the agent terminal delivers SIGINT via forwarded input bytes, not an engine frame; if it exits the agent it classifies `unexpected` for v1.
