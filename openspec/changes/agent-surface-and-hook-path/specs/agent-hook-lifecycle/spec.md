## ADDED Requirements

### Requirement: Hook setup runs before agent spawn

When the orchestrator opens an agent surface, it SHALL run the hook-installation routine before spawning the agent process. The installation SHALL be idempotent: running it when hooks are already installed SHALL produce no change and SHALL NOT return an error.

#### Scenario: Setup runs on open_agent

- **WHEN** `open_agent` is called for an agent surface
- **THEN** the orchestrator SHALL invoke the hook-install routine before issuing the daemon spawn command

#### Scenario: Re-install when hooks already present is a no-op

- **WHEN** `open_agent` is called and the agent's settings file already contains the notify hooks
- **THEN** the hook-install routine SHALL return successfully without modifying the settings file

### Requirement: Hook teardown runs on surface remove

When an agent surface is removed, the orchestrator SHALL run the hook-uninstall routine to remove the notify hooks from the agent's settings file. The uninstall SHALL be idempotent and SHALL coexist safely with user-owned hooks in the same settings file.

#### Scenario: Teardown runs on remove

- **WHEN** `remove` is called for an agent surface
- **THEN** the orchestrator SHALL invoke the hook-uninstall routine for that surface's agent home

#### Scenario: User-owned hooks are preserved on uninstall

- **WHEN** the hook-uninstall routine runs and the settings file contains hooks not belonging to the notify mechanism
- **THEN** those user-owned hooks SHALL remain in the settings file after uninstall

#### Scenario: Uninstall when no hooks present is a no-op

- **WHEN** `remove` is called and no notify hooks are present in the settings file
- **THEN** the hook-uninstall routine SHALL return successfully without error

### Requirement: Hook setup coexists with user-owned hooks

The hook-installation routine SHALL identify its own entries by a stable marker and SHALL NOT remove or modify any hook entry that does not carry that marker.

#### Scenario: User hooks survive install

- **WHEN** the hook-install routine runs on a settings file that contains user-authored hook entries
- **THEN** those entries SHALL remain present and unmodified after install

#### Scenario: Legacy entries migrated on install

- **WHEN** the hook-install routine encounters a pre-existing entry using the older delivery mechanism (identified by a distinct marker)
- **THEN** the routine SHALL replace only that entry with the current notify-binary entry, leaving all other entries untouched
