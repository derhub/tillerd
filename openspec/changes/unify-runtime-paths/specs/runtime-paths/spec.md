## ADDED Requirements

### Requirement: Single runtime directory resolver

The library SHALL resolve the tillerd runtime directory from the `TILLERD_DIR` environment variable
when set, falling back to `~/.tillerd`, and SHALL be the only implementation of that resolution in the
workspace. Every component that needs the runtime directory SHALL obtain it from this library.

#### Scenario: Environment override

- **WHEN** `TILLERD_DIR` is set
- **THEN** the resolved runtime directory is exactly that path

#### Scenario: Default when unset

- **WHEN** `TILLERD_DIR` is not set
- **THEN** the resolved runtime directory is `~/.tillerd`

#### Scenario: One resolver

- **WHEN** any component needs the runtime directory
- **THEN** it calls this library
- **AND** no component defines its own runtime-directory resolver

### Requirement: Runtime-layout path builders

The library SHALL build every runtime-layout path under the resolved runtime directory: the daemon
socket, the gate socket, the daemon manifest, and the product store. The file names SHALL be defined
only in this library, and components SHALL NOT assemble these paths by joining string literals
themselves.

#### Scenario: Paths share the runtime directory

- **WHEN** the daemon socket, gate socket, daemon manifest, and product store paths are requested
- **THEN** each is the corresponding file name joined onto the resolved runtime directory
- **AND** all four share the runtime directory as their parent

#### Scenario: File names defined once

- **WHEN** a component needs the daemon socket, gate socket, daemon manifest, or product store path
- **THEN** it obtains it from this library
- **AND** it does not hardcode the file name or join it onto a directory itself

### Requirement: Service binary resolution by precedence

The library SHALL resolve each service binary (daemon, gate, notify) by a defined precedence:
the binary-specific override environment variable when it names an existing file, then `bin/<name>`
or the cargo build output `target/release/<name>` or `target/debug/<name>` under the working
directory or any ancestor, then `~/.local/bin/<name>`. It SHALL return no result when none exists.
This SHALL be the only binary resolver in the workspace.

#### Scenario: Override wins when present

- **WHEN** the override environment variable names an existing file
- **THEN** that path is returned

#### Scenario: Override skipped when missing

- **WHEN** the override environment variable names a path that does not exist
- **THEN** that path is not returned
- **AND** resolution continues to the discovery fallbacks

#### Scenario: Cargo build output discovered without env

- **WHEN** no override is set and the binary exists under an ancestor's `target/release` or `bin/`
- **THEN** that discovered path is returned

#### Scenario: None when absent

- **WHEN** the binary exists in none of the searched locations
- **THEN** no path is returned

### Requirement: Single source for the environment-variable surface

The library SHALL define the `TILLERD_*` environment-variable names it governs (the runtime
directory and the service-binary overrides) as named constants, and components SHALL reference those
constants rather than repeating the literal variable names.

#### Scenario: Names referenced, not repeated

- **WHEN** a component reads a governed `TILLERD_*` variable
- **THEN** it does so through this library's constant or accessor
- **AND** it does not string-literal the variable name
