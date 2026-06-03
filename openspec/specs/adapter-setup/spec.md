# adapter-setup Specification

## Purpose
TBD - created by archiving change adapter-setup-contract. Update Purpose after archive.
## Requirements
### Requirement: Adapter setup contract

The SDK SHALL define a `defineSetup` contract by which an adapter declares its host setup as two
procedures — `install(context)` and `uninstall(context)` — that the host invokes directly. The
adapter SHALL own the full install and uninstall procedure and the agent-specific decisions (which
file, which keys, idempotency, ordering). The host SHALL supply a setup context carrying the
resolved notify command, the resolved agent-home, a logger, and a filesystem capability
(`readText`, `writeAtomic`, `backup`, `exists`). The contract SHALL be a host-invoked procedure,
not a value the host applies on the adapter's behalf.

The generic file mechanics — read, timestamped backup, and atomic write — SHALL be provided by the
host through the filesystem capability and SHALL NOT be reimplemented by each adapter. The adapter
SHALL compute the next file content and invoke the capability to persist it. `defineSetup` SHALL be
a pure typing helper, and the adapter's procedures SHALL touch no host primitive directly, reaching
the filesystem only through the injected capability and assembling paths from the supplied
agent-home with pure string operations.

#### Scenario: Adapter declares setup as install and uninstall procedures

- **WHEN** an adapter provides its setup through `defineSetup`
- **THEN** it SHALL expose `install(context)` and `uninstall(context)` procedures, and the host
  SHALL invoke them directly rather than receiving a plan to apply

#### Scenario: Host supplies the setup context and filesystem capability

- **WHEN** the host invokes the adapter's `install` or `uninstall`
- **THEN** it SHALL pass a setup context carrying the resolved notify command, the resolved
  agent-home, a logger, and a filesystem capability, and the adapter SHALL reach the filesystem only
  through that capability and assemble the settings path from the agent-home

#### Scenario: Install is idempotent

- **WHEN** the host invokes `install` and the adapter's setup is already present
- **THEN** the procedure SHALL complete without adding a duplicate, leaving the prior setup intact

#### Scenario: Uninstall is idempotent

- **WHEN** the host invokes `uninstall` and the adapter's setup is already absent
- **THEN** the procedure SHALL complete without error and SHALL make no change

#### Scenario: Setup write preserves the prior file

- **WHEN** `install` or `uninstall` modifies the agent settings file
- **THEN** the adapter SHALL invoke the host filesystem capability to back up the prior file and
  write the new contents atomically, leaving no partial or temporary file on completion, with the
  backup and atomic write owned by the host capability rather than the adapter

