## 1. Package scaffold

- [x] 1.1 Create `apps/cli` package (package.json, tsconfig) wired into the workspace and turbo pipeline
- [x] 1.2 Add `@clack/prompts` dependency; depend on `@athing/adapter-claude-code` and `@athing/platform-bun`
- [x] 1.3 Add a `bin/` entry (e.g. `bin/athing`) that executes the CLI entry point

## 2. Command routing

- [x] 2.1 Implement the entry point that reads the positional subcommand and dispatches to handlers
- [x] 2.2 Implement usage/help text and non-zero exit for missing or unknown subcommands
- [x] 2.3 Add a TTY/flag helper that decides interactive vs non-interactive mode
- [x] 2.4 Validate arguments per-subcommand (strict): reject unknown flags, cross-command flags, and stray positionals; support `-h`/`--help`

## 3. Install / uninstall

- [x] 3.1 Resolve the notify command (`prepareNotifyScript`/`notifyCommand`) and build a `SetupContext` via `buildSetupContext`
- [x] 3.2 Implement `install` handler: optional clack confirm on TTY, `--yes`/non-TTY skips prompt, `await setup.install(ctx)`, report installed events
- [x] 3.3 Implement `uninstall` handler: `await setup.uninstall(ctx)`, reporting nothing-to-remove case

## 4. Daemon status

- [x] 4.1 Implement `status` handler using `readManifest` + `isAlive`, mapping running/stale/absent states
- [x] 4.2 Implement human-formatted output and `--json` output; set exit codes per state

## 5. Tests

- [x] 5.1 Test command routing: known, missing, and unknown subcommands and their exit codes
- [x] 5.2 Test install idempotency and uninstall preserves unrelated hook entries (temp settings file)
- [x] 5.3 Test status across running / stale / absent states and `--json` shape (fake manifest + pid)
- [x] 5.4 Test non-interactive fallback: prompts are skipped when stdin is not a TTY or `--yes` is set

## 6. Docs

- [x] 6.1 Add a short usage section to the package README covering the three subcommands and flags
