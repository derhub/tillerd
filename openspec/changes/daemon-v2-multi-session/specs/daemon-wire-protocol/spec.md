## ADDED Requirements

### Requirement: Binary frame format

All messages between daemon and client SHALL use a length-prefixed binary frame format: a 4-byte big-endian unsigned integer encoding the total payload length, followed by a JSON metadata object, followed by an optional raw binary body. The JSON metadata and binary body are separated by a single newline byte (`0x0A`); if no binary body is present the newline is omitted.

#### Scenario: Frame with binary payload

- **WHEN** the daemon emits a data frame carrying PTY output bytes
- **THEN** the frame SHALL consist of: 4-byte length, JSON metadata (`{"type":"data","sessionId":"…","bodyLen":<n>}`), newline, then exactly `<n>` raw bytes

#### Scenario: Frame without binary payload

- **WHEN** the daemon sends a control message (spawn-ack, exit, error)
- **THEN** the frame SHALL consist of: 4-byte length, JSON metadata only, with no trailing newline or body

#### Scenario: Incomplete frame held in buffer

- **WHEN** a TCP segment delivers fewer bytes than the declared frame length
- **THEN** the receiver SHALL buffer the bytes and wait for the remainder before processing the frame

### Requirement: Protocol version negotiation handshake

Upon connection the client SHALL send a `hello` frame declaring the set of protocol versions it supports. The daemon SHALL reply with a `hello-ack` frame naming the chosen version and its own binary version string. If no mutually supported version exists the daemon SHALL reply with an `error` frame and close the connection.

#### Scenario: Compatible versions

- **WHEN** a client connects and sends `{"type":"hello","versions":[1]}`
- **THEN** the daemon SHALL reply `{"type":"hello-ack","version":1,"daemonVersion":"<semver>"}` and accept subsequent frames

#### Scenario: Incompatible versions

- **WHEN** a client connects and sends `{"type":"hello","versions":[99]}`
- **THEN** the daemon SHALL reply `{"type":"error","code":"EVERSION","message":"…"}` and close the socket

#### Scenario: Message before handshake rejected

- **WHEN** a client sends any frame other than `hello` before completing the handshake
- **THEN** the daemon SHALL reply with an `error` frame and close the connection

### Requirement: Typed frame catalogue

The protocol SHALL define the following frame types. Frames marked C→D are sent by clients; D→C are sent by the daemon; D↔C travel both directions.

| Type          | Direction | Purpose                         |
| ------------- | --------- | ------------------------------- |
| `hello`       | C→D       | Handshake initiation            |
| `hello-ack`   | D→C       | Handshake response              |
| `spawn`       | C→D       | Create a new session            |
| `spawn-ack`   | D→C       | Session created, includes pid   |
| `kill`        | C→D       | Terminate a session             |
| `list`        | C→D       | Request live session ids        |
| `list-ack`    | D→C       | Response to list                |
| `subscribe`   | C→D       | Subscribe to session output     |
| `unsubscribe` | C→D       | Unsubscribe from session output |
| `input`       | C→D       | Raw bytes to PTY stdin          |
| `resize`      | C→D       | PTY window resize               |
| `interrupt`   | C→D       | Send SIGINT to PTY process      |
| `data`        | D→C       | PTY output bytes (binary body)  |
| `exit`        | D→C       | Session exited                  |
| `hook`        | D→C       | Hook event relay                |
| `ack`         | C→D       | Flow-control credit return      |
| `error`       | D↔C       | Typed error                     |

#### Scenario: Unknown frame type dropped

- **WHEN** either side receives a frame with an unrecognised `type` value
- **THEN** it SHALL log the unknown type and continue processing subsequent frames without closing the connection
