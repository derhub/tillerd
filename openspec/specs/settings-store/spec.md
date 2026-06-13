# settings-store Specification

## Purpose

Defines the settings store: a host-agnostic, scoped (global / project) key→value store for user settings, persisted durably by the orchestrator and reached through a web-safe client so any host satisfies the same contract. Project scope resolves over global, and it provides generic confirmation-suppression ("don't ask again") storage.

## Requirements

### Requirement: Scoped settings persistence

The system SHALL persist user settings as key→value entries under a scope that is either global or bound to a specific project, durably across restarts. Values SHALL be structured (JSON-serializable). A setting SHALL be uniquely identified by the triple (scope, project, key).

#### Scenario: Global setting round-trips

- **WHEN** a global setting is written for a key and then read back for the same key
- **THEN** the stored value is returned unchanged

#### Scenario: Project-scoped setting round-trips

- **WHEN** a setting is written for a key scoped to a project and then read back for that key and project
- **THEN** the stored value is returned unchanged

#### Scenario: Overwriting a setting replaces the value

- **WHEN** a setting is written twice for the same scope, project, and key
- **THEN** a read returns the most recently written value, with no duplicate entries

#### Scenario: Settings survive a restart

- **WHEN** a setting is written, the store is closed, and the store is reopened against the same backing data
- **THEN** reading the key returns the previously written value

### Requirement: Project override resolves over global

The system SHALL resolve a setting for a project by returning the project-scoped value when present, and otherwise falling back to the global value for the same key.

#### Scenario: Project value takes precedence

- **WHEN** both a global and a project-scoped value exist for a key and the key is resolved for that project
- **THEN** the project-scoped value is returned

#### Scenario: Falls back to global on project miss

- **WHEN** only a global value exists for a key and the key is resolved for a project
- **THEN** the global value is returned

#### Scenario: Unknown key resolves to absent

- **WHEN** a key with no global or project value is resolved
- **THEN** an absent result is returned rather than an error

### Requirement: Listing settings by scope

The system SHALL return all settings for a given scope so a consumer can enumerate current configuration.

#### Scenario: List returns written entries

- **WHEN** several global settings have been written and the global scope is listed
- **THEN** every written key and its value is present in the result

### Requirement: Host-agnostic settings access

The system SHALL expose settings reads and writes through a host-agnostic interface so that any host (desktop or a future server/web host) satisfies the same contract without changing settings behavior. The renderer-facing client SHALL be web-safe (no host-only runtime APIs).

#### Scenario: Renderer reads and writes through the port

- **WHEN** the renderer sets a setting and then reads it through the settings interface
- **THEN** the value round-trips, regardless of which host adapter backs the interface

### Requirement: Reusable confirmation-suppression preference

The system SHALL provide generic keyed boolean storage so a UI flow can record that the user chose not to be asked again for a given confirmation, and later read that choice.

#### Scenario: Suppression choice is recorded and read

- **WHEN** a "don't ask again" choice is recorded for a named confirmation and later read
- **THEN** the recorded boolean is returned

#### Scenario: Unset confirmation defaults to not-suppressed

- **WHEN** a confirmation that was never recorded is read
- **THEN** the result indicates the confirmation is not suppressed
