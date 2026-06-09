# cli-controller Specification

## Purpose

A scriptable controller/installer command-line tool. It routes a fixed set of subcommands to
install and uninstall the agent hooks (driving the adapter's setup procedures) and to report
daemon status, with interactive prompts on a TTY and a non-interactive path for CI.

## Requirements

### Requirement: Command routing

The CLI SHALL parse its arguments with Bun's `util.parseArgs` and route to one of a fixed set
of subcommands. Unknown or missing subcommands SHALL print usage and exit non-zero.

#### Scenario: Known subcommand routes

- **WHEN** the CLI is invoked as `tillerd <subcommand>` where `<subcommand>` is one of
  `install`, `uninstall`, or `status`
- **THEN** the CLI runs that subcommand's handler

#### Scenario: No subcommand prints usage

- **WHEN** the CLI is invoked with no subcommand
- **THEN** the CLI prints usage listing the available subcommands
- **AND** exits with a non-zero status

#### Scenario: Unknown subcommand is rejected

- **WHEN** the CLI is invoked with a subcommand that is not recognized
- **THEN** the CLI prints an error naming the unknown subcommand and the available subcommands
- **AND** exits with a non-zero status

### Requirement: Help

The CLI SHALL print usage and exit zero when `-h` or `--help` is passed.

#### Scenario: Help flag

- **WHEN** the CLI is invoked with `-h` or `--help`
- **THEN** the CLI prints usage to stdout
- **AND** exits zero

### Requirement: Argument validation

The CLI SHALL reject unknown flags, flags that are not valid for the chosen subcommand, and
unexpected positional arguments. On any such input it SHALL print an error and usage and exit
non-zero, performing no side effects.

#### Scenario: Unknown flag

- **WHEN** a subcommand is invoked with a flag it does not define
- **THEN** the CLI prints an error and usage
- **AND** exits non-zero without performing the subcommand's action

#### Scenario: Flag scoped to another subcommand

- **WHEN** a flag valid for one subcommand (e.g. `--json`) is passed to a subcommand that does
  not define it (e.g. `install`)
- **THEN** the CLI rejects it with an error and usage
- **AND** exits non-zero without writing

#### Scenario: Unexpected positional argument

- **WHEN** a subcommand is given an extra positional argument
- **THEN** the CLI prints an error naming the argument and usage
- **AND** exits non-zero

### Requirement: Hook installation

The `install` subcommand SHALL install the agent hooks into the agent settings file by reusing
the adapter's hook installer, and SHALL be idempotent.

#### Scenario: Install on a fresh environment

- **WHEN** `tillerd install` is run and no agent hooks are present
- **THEN** the hooks are written to the agent settings file
- **AND** the CLI reports which hook events were installed
- **AND** exits zero

#### Scenario: Install when already installed

- **WHEN** `tillerd install` is run and the hooks are already present
- **THEN** no duplicate hooks are written
- **AND** the CLI reports that hooks are already installed
- **AND** exits zero

#### Scenario: Interactive confirmation on a TTY

- **WHEN** `tillerd install` is run attached to a TTY without a confirmation flag
- **THEN** the CLI prompts for confirmation before writing
- **AND** declining the prompt makes no changes and exits non-zero

#### Scenario: Non-interactive install

- **WHEN** `tillerd install --yes` is run, or stdin is not a TTY
- **THEN** the CLI installs without prompting

### Requirement: Hook uninstallation

The `uninstall` subcommand SHALL remove only the hooks this tool installed from the agent
settings file, leaving unrelated hook entries intact, and SHALL be idempotent.

#### Scenario: Uninstall removes managed hooks

- **WHEN** `tillerd uninstall` is run and managed hooks are present
- **THEN** only the managed hook entries are removed from the settings file
- **AND** unrelated hook entries are preserved
- **AND** exits zero

#### Scenario: Uninstall when nothing installed

- **WHEN** `tillerd uninstall` is run and no managed hooks are present
- **THEN** the settings file is unchanged
- **AND** the CLI reports there was nothing to remove
- **AND** exits zero

### Requirement: Daemon status

The `status` subcommand SHALL report whether the daemon is running by reading the daemon
manifest and probing process liveness, and SHALL distinguish running, stale, and absent states.

#### Scenario: Daemon running

- **WHEN** `tillerd status` is run and the manifest exists and its pid is alive
- **THEN** the CLI reports the daemon as running with its pid and version
- **AND** exits zero

#### Scenario: Stale manifest

- **WHEN** `tillerd status` is run and the manifest exists but its pid is not alive
- **THEN** the CLI reports the daemon as not running (stale manifest)
- **AND** exits non-zero

#### Scenario: No manifest

- **WHEN** `tillerd status` is run and no manifest exists
- **THEN** the CLI reports the daemon as not running
- **AND** exits non-zero

### Requirement: Machine-readable status output

The `status` subcommand SHALL support a `--json` flag that prints the status as a single JSON
object instead of human-formatted text.

#### Scenario: JSON output

- **WHEN** `tillerd status --json` is run
- **THEN** the CLI prints one JSON object describing daemon state (running, pid, version)
- **AND** prints no human-formatted decoration
