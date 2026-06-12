# Agent Adapter

## ADDED Requirements

### Requirement: The agent adapter wraps the generic spawn with the agent lifecycle

When launching an agent surface the agent adapter SHALL, in order: subscribe to the hook fan-out by
the surface identifier before spawning so no hook event is missed; install the agent's notify
hooks; spawn the command supplied by the launch item through the generic spawn; then drain the hook
stream into status and content events emitted over the event sink. On surface removal the adapter
SHALL cancel the subscription and uninstall the agent's hooks.

#### Scenario: Subscription precedes spawn
- **WHEN** an agent surface is launched
- **THEN** the hook subscription is established before the command is spawned

#### Scenario: Hook events become status and content
- **WHEN** the agent emits hook events after launch
- **THEN** the adapter routes them to status and content events tagged with the surface identifier

#### Scenario: Removal tears down the agent lifecycle
- **WHEN** an agent surface is removed
- **THEN** the subscription is cancelled and the agent's hooks are uninstalled

#### Scenario: A subscription failure surfaces a typed error
- **WHEN** the hook fan-out refuses the subscription at launch
- **THEN** the surface launch fails with a typed error and no agent process is spawned

### Requirement: The agent definition holds only adapter semantics

The agent definition SHALL carry only how to interpret and control a running agent: hook-to-status
and hook-to-content parsing, the interrupt sequence, the acceptable version range, and hook
install/teardown. It SHALL NOT carry how to launch the agent; the launch command comes from the
command library by way of the launch item.

#### Scenario: The launch command is not taken from the definition
- **WHEN** an agent surface is launched
- **THEN** its executable and arguments come from the launch item's command, not from the agent definition

#### Scenario: Status mapping is owned by the definition
- **WHEN** a hook event of a given kind is received
- **THEN** the agent definition maps it to the contract status
