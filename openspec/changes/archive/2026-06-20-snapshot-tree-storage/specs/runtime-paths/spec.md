## MODIFIED Requirements

### Requirement: Runtime-layout path builders

The library SHALL build every runtime-layout path under the resolved runtime directory: the daemon
socket, the gate socket, the daemon manifest, and the data root. The file names and directory names
SHALL be defined only in this library, and components SHALL NOT assemble these paths by joining
string literals themselves.

#### Scenario: Paths share the runtime directory

- **WHEN** the daemon socket, gate socket, daemon manifest, and data root paths are requested
- **THEN** each is the corresponding name joined onto the resolved runtime directory
- **AND** all four share the runtime directory as their parent

#### Scenario: Data root is a directory

- **WHEN** the data root path is requested
- **THEN** the returned path MUST be a relocatable directory (not a single file)
- **AND** the directory SHALL contain a `workspaces/` sub-directory as its root namespace

#### Scenario: File names defined once

- **WHEN** a component needs the daemon socket, gate socket, daemon manifest, or data root path
- **THEN** it obtains it from this library
- **AND** it does not hardcode the name or join it onto a directory itself
