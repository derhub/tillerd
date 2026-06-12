# Command Library

## ADDED Requirements

### Requirement: Prebuilt commands are seeded idempotently

The store SHALL seed its prebuilt command-library entries on first open and SHALL NOT create
duplicates on repeated opens or under concurrent opens.

#### Scenario: Repeated open does not duplicate
- **WHEN** the store is opened twice
- **THEN** each prebuilt command exists exactly once

#### Scenario: Concurrent open does not duplicate
- **WHEN** two opens seed the prebuilt commands concurrently
- **THEN** each prebuilt command exists exactly once

### Requirement: Command-library entries support full CRUD

The store SHALL support creating, getting, listing, and deleting command-library entries.

#### Scenario: Create then get
- **WHEN** a command is created and then fetched by its identifier
- **THEN** the stored command is returned

#### Scenario: Delete removes the entry
- **WHEN** a command is deleted
- **THEN** a subsequent get for that identifier returns nothing
