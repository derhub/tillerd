# cli

Controller/installer for the agent integration. A small command surface (parsed with
Bun's `util.parseArgs`) over the existing setup contract and daemon manifest.

## Usage

```
athing install [--yes]    Install agent hooks into the agent settings file
athing uninstall          Remove the hooks this tool installed
athing status [--json]    Report whether the daemon is running
```

### Flags

- `--yes` — skip the interactive confirmation on `install` (also implied when stdin is not a
  TTY, so the CLI is fully scriptable in CI).
- `--json` — `status` prints a single JSON object instead of human-formatted text.
- `-h`, `--help` — print usage.

### Exit codes

- `install` / `uninstall` — `0` on success; `install` returns non-zero when the TTY confirmation
  is declined or the notify command cannot be resolved.
- `status` — `0` when the daemon is running, non-zero when stale (manifest present, pid dead) or
  absent (no manifest).

## How it works

- `install` / `uninstall` resolve the notify command (the same path the server uses) and drive
  the adapter's `setup` procedures with a host-built `SetupContext` from `@athing/platform-bun`.
- `status` reads `~/.athing/daemon.json` and probes process liveness.
