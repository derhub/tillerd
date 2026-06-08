## Context

Hook installation and daemon liveness checks already exist as reusable pieces:

- The adapter owns its hook setup via the `setup` definition in `@athing/adapter-claude-code`
  (`setup.install` / `setup.uninstall`), each an async procedure taking a `SetupContext`. They
  are idempotent (they detect a hook marker) and write atomically with a backup.
- The host supplies the `SetupContext` from `@athing/platform-bun`: `buildSetupContext(notifyCommand, logger)`
  wires `agentHome()` and the `setupFs` filesystem capability; the notify command is resolved by
  `prepareNotifyScript()` / `notifyCommand` (the same path the server uses).
- `readManifest` and `isAlive` in `@athing/platform-bun` read `~/.athing/daemon.json` (pid,
  version) and probe `process.kill(pid, 0)`.

Today these are only invoked as a side effect of booting `apps/server`. There is no standalone
operator entry point. This change adds a thin `apps/cli` that composes those existing pieces
behind a small command surface. The CLI is a host-layer app, so it may use Node/Bun APIs
directly (the Web-API-only rule applies to `sdk`/`engine`, not apps).

## Goals / Non-Goals

**Goals:**

- A scriptable, first-class `install` / `uninstall` / `status` entry point.
- Reuse existing install and liveness logic without modifying it.
- Fully non-interactive operation for CI (flags + non-TTY fallback), with friendly interactive
  prompts on a TTY.
- Minimal dependency footprint.

**Non-Goals:**

- No TUI / live-updating dashboard (the tool is request-response).
- No session control, attach, or agent I/O (controller/installer scope only).
- No starting/stopping the daemon (status reports only; lifecycle stays with the server/host).
- No changes to the install/liveness functions themselves.

## Decisions

### Argument parsing: Bun's `util.parseArgs`, not a CLI framework

The command surface is three subcommands with a couple of boolean flags. `parseArgs` is built
into the runtime, zero dependency, and sufficient. Alternatives: Commander.js (the ecosystem
standard, but adds a dependency and auto-help machinery this small surface does not need);
citty (UnJS, tiny, but still a dependency). Chosen `parseArgs` to keep the footprint at a single
runtime dependency. Subcommand is read positionally; flags parsed per-subcommand.

### Interactivity: `@clack/prompts`, guarded by TTY detection

`@clack/prompts` provides the confirm prompt and spinner for the install flow. Because clack
prompts hang or misbehave without a TTY, every prompt is gated: if `--yes`/explicit flags are
given, or `process.stdin.isTTY` is false, the CLI takes the non-interactive path and never
prompts. This satisfies the "every prompt has a flag equivalent and non-TTY fallback"
requirement.

### Install via the setup contract, host-built context

The CLI does not write settings itself. It builds a `SetupContext` with
`buildSetupContext(notifyCommand, logger)` from `@athing/platform-bun` and calls the adapter's
`setup.install(ctx)` / `setup.uninstall(ctx)`. The notify command is resolved the same way the
server does — `prepareNotifyScript()` / `notifyCommand` (the prepared notify script /
`bin/athing-notify`) — so installed hooks behave identically regardless of who installed them.
Both procedures are async, so handlers `await` them.

### Status semantics: three states, exit codes carry the verdict

`status` maps to three outcomes — running (manifest present + pid alive), stale (manifest present

- pid dead), absent (no manifest). Running exits zero; stale and absent exit non-zero so scripts
  can branch on exit code. `--json` emits a single object for machine consumers; without it, a
  short human line.

## Risks / Trade-offs

- [Importing the `setup` definition from the adapter couples the CLI to a specific adapter] →
  Acceptable for v1 (single adapter). The setup contract lives in `@athing/sdk`, so adding
  adapter selection later means choosing which adapter's `setup` to drive — no CLI rework.
- [`parseArgs` gives no auto-generated help] → Hand-write a short usage string; the surface is
  small enough that this is cheaper than a framework dependency.
- [Liveness via `process.kill(pid, 0)` cannot detect a hung-but-alive daemon] → Out of scope;
  `status` reports process existence, not socket health, in v1. Socket reachability can be added
  later behind the same command.

## Migration Plan

Additive: a new package and a new `bin` entry. No existing behavior changes, so no rollback
concern beyond removing the package. The server continues to install hooks on boot independently.
