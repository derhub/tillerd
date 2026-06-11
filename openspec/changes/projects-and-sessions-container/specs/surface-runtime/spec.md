## MODIFIED Requirements

### Requirement: Detach preserves the pseudo-terminal; removal terminates it

A proxy detach caused by host shutdown or a dropped client SHALL leave the pseudo-terminal running
in the daemon so the surface can resume; the pseudo-terminal's lifetime SHALL follow the surface,
not the client connection. Removing the surface SHALL terminate its pseudo-terminal and release the
proxy. Surface creation SHALL require a caller-supplied `session_id`; the surface-runtime SHALL NOT
mint an implicit session when creating a surface. Soft-deleting a surface via the session container
SHALL mark the surface record as archived without terminating the pseudo-terminal; hard removal via
the session container SHALL terminate the pseudo-terminal.

#### Scenario: Detach keeps the pseudo-terminal alive

- **WHEN** the host shuts down or a client disconnects
- **THEN** the proxy detaches and the pseudo-terminal keeps running in the daemon

#### Scenario: Removal terminates the pseudo-terminal

- **WHEN** the surface is removed via a hard-remove operation
- **THEN** its pseudo-terminal is terminated and the proxy is released

#### Scenario: Surface creation requires caller-supplied session id

- **WHEN** a create-surface request is received
- **THEN** the surface-runtime uses the caller-supplied `session_id` to associate the surface record and does not create a new session

#### Scenario: Soft-delete does not terminate pseudo-terminal

- **WHEN** a surface is soft-deleted through a session container archive operation
- **THEN** the surface record's `deleted_at` is set and the pseudo-terminal is not terminated
