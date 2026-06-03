## Why

Installing the agent hooks and checking whether the background daemon is running are
prerequisites for using the SDK, yet today they only happen as a side effect of booting the
server. There is no first-class, scriptable entry point an operator can run to set up an
environment or confirm the daemon is healthy. A small controller/installer CLI fills that gap
and keeps setup logic out of the server boot path.

## What Changes

- Add a new `apps/cli` package: a controller/installer command-line tool.
- Command surface parsed with Bun's built-in `util.parseArgs` (zero added parser
  dependency); interactive flows use `@clack/prompts`.
- **Hook installer commands**: install and uninstall the agent hooks into the agent settings
  file by driving the adapter's `setup` procedures (`setup.install`/`setup.uninstall`) with a
  host-built `SetupContext`. Interactive confirmation when run on a TTY; `--yes` / explicit flags
  for non-interactive (CI) use.
- **Daemon status command**: report whether the daemon is running (pid, alive, version, socket
  reachability) by reading the daemon manifest and probing liveness.
- Every interactive prompt has a flag equivalent and a non-TTY fallback so the CLI is fully
  scriptable.
- A single binary entry registered in `bin/` for invocation.

## Capabilities

### New Capabilities

- `cli-controller`: a command-line controller/installer that routes subcommands, installs and
  uninstalls agent hooks, and reports daemon status. Covers argument parsing, interactive vs
  non-interactive behavior, and the exit-code contract.

### Modified Capabilities

<!-- None: hook install/uninstall and daemon manifest/liveness are reused as-is, not changed. -->

## Impact

- New package `apps/cli` (Bun, TypeScript) added to the workspace.
- New dependency: `@clack/prompts`. No CLI-framework dependency (uses Bun's `util.parseArgs`).
- Reuses the `setup` procedures from `@athing/adapter-claude-code` and, from
  `@athing/platform-bun`, `buildSetupContext` + the notify-command resolver
  (`prepareNotifyScript`/`notifyCommand`) for install/uninstall, plus `readManifest`/`isAlive`
  for status — no changes to those modules.
- New `bin/` entry for the CLI; root `package.json`/turbo pipeline gains the new package.
- No change to server, engine, sdk, or daemon runtime behavior.
