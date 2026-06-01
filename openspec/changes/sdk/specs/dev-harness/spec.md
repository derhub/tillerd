## ADDED Requirements

### Requirement: Server composition root

`apps/server` SHALL be the composition root: it imports the engine, injects a concrete adapter, and exposes a session to clients. It SHALL depend on the engine, the adapter, and the sdk; the engine SHALL NOT depend on the server.

#### Scenario: Adapter injected at the root

- **WHEN** the server starts
- **THEN** it SHALL create an engine instance and inject the `claudeCode` adapter, with the engine unaware of which adapter it received

### Requirement: Session exposed over WebSocket and HTTP

The server SHALL expose a session over a WebSocket (carrying raw terminal bytes, status, and content) and HTTP, and SHALL accept client input, resize, interrupt, and prompt submission.

#### Scenario: Stream session events to a client

- **WHEN** a client connects to a session over the WebSocket
- **THEN** the server SHALL forward the session's data, status, and content events to the client

#### Scenario: Forward client actions

- **WHEN** a client sends a prompt, raw input, resize, or interrupt over the connection
- **THEN** the server SHALL invoke the corresponding session operation on the engine

### Requirement: Validated wire messages

Messages crossing the client/server boundary SHALL be validated against a defined schema; malformed messages SHALL be rejected.

#### Scenario: Reject malformed message

- **WHEN** the server receives a message that does not match the wire schema
- **THEN** it SHALL reject the message and SHALL NOT act on it

### Requirement: Web UI vertical slice

`apps/ui` SHALL be a single-page app that renders the session terminal from the byte stream, shows the session status, and displays structured content. It SHALL depend on the sdk types and the network, with no code dependency on the engine.

#### Scenario: Render the terminal

- **WHEN** the UI receives the session's byte stream
- **THEN** it SHALL render a faithful terminal view and send user keystrokes and resize back to the server

#### Scenario: Show status and content

- **WHEN** status and content events arrive
- **THEN** the UI SHALL display the current status and the structured content (such as tool calls and usage)
