## ADDED Requirements

### Requirement: Agent engine runs inside the renderer

The desktop application SHALL host the agent engine and adapter within the renderer, driving
them through the injected daemon-transport and file-read contracts, with no separate backend
agent process on the desktop path.

#### Scenario: Driving a session without a backend agent process

- **WHEN** the user starts an agent session in the desktop application
- **THEN** the engine runs within the renderer, driving the session through the injected contracts
- **AND** no separate backend agent process is involved

### Requirement: Engine constructed from native startup values

The renderer SHALL construct the engine from the values resolved by the native core — the
bridged daemon transport, the file-read contract, a logger, the hook ingress socket path, the
resolved agent home, and the resolved agent command — at startup, and SHALL supply a working
directory on every session start.

#### Scenario: Constructing the engine at startup

- **WHEN** the application has resolved its startup values
- **THEN** the renderer constructs the engine with the native transport, the native file-read
  contract, a logger, the hook ingress socket path, the resolved agent home, and the resolved
  agent command before starting any session

#### Scenario: Supplying the working directory per session

- **WHEN** the renderer starts or reconnects a session
- **THEN** it supplies the session's working directory
- **AND** a session start without a working directory surfaces a typed error rather than starting

### Requirement: Pluggable transport selection

The renderer SHALL select the native transport when running as the desktop application and the
network transport when running as the web deployment, behind one transport abstraction, with
identical user-facing behavior.

#### Scenario: Selecting the transport per host

- **WHEN** the renderer runs inside the desktop application
- **THEN** it uses the native transport
- **AND** the same renderer running as a web deployment uses the network transport, with
  identical user-facing behavior
