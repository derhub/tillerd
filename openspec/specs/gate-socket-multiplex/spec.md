# gate-socket-multiplex Specification

## Purpose

The gate's single front-door socket: one named Unix domain socket, demultiplexed by a per-connection route preamble. Defines the preamble frame and its route set, the demux, the centralized route-to-credential policy, the per-route post-preamble lifecycle (including the MCP protocol upgrade), and the derived single socket path. Subsumes the previously per-face socket topology for the hook, tool, subscribe, and admin faces.

## Requirements

### Requirement: Single front-door socket at a derived path

The gate SHALL expose exactly one named Unix domain socket at a deterministic path derived from the runtime directory (`$ATHING_DIR/gate.sock`). It SHALL NOT bind a separate socket per face, SHALL NOT open any TCP port, and SHALL NOT publish an address file, because the path is derivable from the runtime directory. The socket SHALL remain valid for as long as the gate runs, across engine host process restarts.

#### Scenario: One socket at a derivable path

- **WHEN** the gate starts
- **THEN** it SHALL bind only `$ATHING_DIR/gate.sock`, and that path SHALL be usable while the gate runs, with no per-face socket and no address file to publish or read

#### Scenario: No network port

- **WHEN** the gate binds its faces
- **THEN** it SHALL bind only the single named Unix domain socket and SHALL NOT open any TCP port

#### Scenario: Socket survives engine restart

- **WHEN** the engine host process exits and restarts
- **THEN** the gate socket SHALL still accept connections from running sessions without reconfiguration

### Requirement: Route preamble opens every connection

Every connection to the gate socket SHALL begin with exactly one length-prefixed preamble frame, encoded with the gate's shared frame codec, carrying a route selector drawn from a fixed set — `Hook`, `Tool`, `Subscribe`, `Admin`, `Mcp` — together with the session id, an optional bearer token, and a wire version. The gate SHALL read this preamble before any face-specific exchange and SHALL demultiplex the connection to the selected route. A connection whose preamble is malformed, declares an unknown route, or declares an unsupported wire version SHALL be refused without reaching any route.

#### Scenario: Preamble selects the route

- **WHEN** a client connects and sends a preamble frame naming a supported route with a supported wire version
- **THEN** the gate SHALL hand the connection to that route's behavior

#### Scenario: Unknown route is refused

- **WHEN** a connection's preamble names a route outside the fixed set
- **THEN** the gate SHALL refuse the connection and SHALL NOT change any session state

#### Scenario: Malformed or unsupported preamble is refused

- **WHEN** a connection's preamble cannot be decoded or declares an unsupported wire version
- **THEN** the gate SHALL refuse the connection before any face-specific exchange

### Requirement: Per-route demultiplexing preserves each lifecycle

After a valid preamble the connection SHALL behave according to its route, preserving each face's existing exchange shape: `Hook` accepts one or more payload frames fire-and-forget with no reply; `Tool` is a request/response exchange; `Subscribe` negotiates the wire version, acknowledges readiness, then streams session events server-push; `Admin` is a request/response exchange; `Mcp` upgrades the connection to the MCP protocol.

#### Scenario: Hook route does not reply

- **WHEN** a connection on the `Hook` route sends a payload frame
- **THEN** the gate SHALL route it for fan-out and SHALL NOT write a reply on the connection

#### Scenario: Tool and Admin routes reply per request

- **WHEN** a connection on the `Tool` or `Admin` route sends a request frame
- **THEN** the gate SHALL write exactly one response frame for that request

#### Scenario: Subscribe route streams after readiness

- **WHEN** a connection on the `Subscribe` route completes wire-version negotiation
- **THEN** the gate SHALL acknowledge readiness and then stream that session's events until the connection closes

### Requirement: Centralized route-to-credential policy

The gate SHALL enforce a single, centralized policy mapping each route to the credential it requires: `Hook`, `Tool`, and `Mcp` SHALL require a valid per-session token verified against the session registry; `Admin` SHALL require the admin token, which is distinct from any session token; `Subscribe` SHALL require no token. The required credential SHALL be checked before the route's behavior runs. A connection that satisfies only a session token SHALL NOT be admitted to the `Admin` route.

#### Scenario: Session-token route admits a valid session token

- **WHEN** a connection on the `Hook`, `Tool`, or `Mcp` route presents a token that verifies against its session
- **THEN** the gate SHALL admit it to that route

#### Scenario: A session token cannot satisfy the Admin route

- **WHEN** a connection on the `Admin` route presents a valid per-session token but not the admin token
- **THEN** the gate SHALL refuse it before any registry mutation

#### Scenario: A missing or wrong credential is refused before the route runs

- **WHEN** a connection presents a missing or mismatched credential for its route
- **THEN** the gate SHALL refuse it before the route's behavior runs and SHALL NOT change any state

### Requirement: The MCP route upgrades to the MCP protocol after the preamble

For a connection on the `Mcp` route, the gate SHALL verify the preamble's session token, then stop reading gate frames and hand the remaining byte stream to the MCP protocol, so the gate owns only the admission preamble and the bridge from an MCP request to an internal inbound. The MCP exchange — including the initialize handshake and version negotiation — SHALL run over a maintained protocol library, not a hand-rolled wire.

#### Scenario: Verified preamble upgrades to MCP

- **WHEN** a connection on the `Mcp` route presents a valid session token in its preamble
- **THEN** the gate SHALL upgrade the connection to the MCP protocol and serve MCP requests over the remaining stream

#### Scenario: Unverified preamble never upgrades

- **WHEN** a connection on the `Mcp` route presents a missing or invalid session token
- **THEN** the gate SHALL refuse it and SHALL NOT begin the MCP protocol on that connection

### Requirement: Single-listener lifecycle teardown

The gate SHALL participate in graceful shutdown through its single listener: on shutdown it SHALL stop accepting new connections and release the socket, so every route stops together and no per-face listener is left bound.

#### Scenario: Shutdown stops every route at once

- **WHEN** the gate begins graceful shutdown
- **THEN** it SHALL stop accepting new connections on the socket and release it, and no route SHALL continue to accept new connections
