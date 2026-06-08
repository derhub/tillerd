# gate-mcp-routing Specification

## Purpose

The gate's MCP face: a fifth transport face, peer to hook/tool/subscribe/admin, that accepts MCP protocol requests over a configurable loopback-only transport, authenticates them with the per-session bearer token, normalizes them into a distinct MCP inbound kind, routes them through the gate's existing global middleware (observe, auth) unchanged, and publishes the bound endpoint for discovery. This version is the routing layer only — it registers no tool implementations.

## Requirements

### Requirement: MCP ingress face

The gate SHALL expose an MCP ingress face that accepts MCP protocol requests, peer to its
existing transport faces. The face SHALL speak the MCP protocol — including the initialize
handshake and protocol-version negotiation — over a maintained protocol library rather than
a hand-rolled wire, so the gate owns only the bridge from an MCP request to an internal
inbound.

#### Scenario: A compliant client completes the initialize handshake

- **WHEN** an MCP client connects to the face and sends an initialize request with its
  supported protocol version
- **THEN** the face completes the handshake, negotiating a mutually supported protocol
  version, and the connection becomes ready to serve requests

#### Scenario: Version negotiation fails for an unsupported client

- **WHEN** an MCP client requests a protocol version the face cannot satisfy
- **THEN** the face declines the handshake with a protocol error and does not serve
  requests on that connection

### Requirement: Configurable local transport

The face SHALL be reachable over a local transport selected by configuration, supporting at
least a loopback network transport and a local socket transport. The same request-handling
behavior — authentication, normalization, routing — SHALL apply identically regardless of
which transport is bound.

#### Scenario: Configuration selects the bound transport

- **WHEN** the gate starts with a configured MCP transport
- **THEN** the face binds only that transport and serves MCP requests on it

#### Scenario: Behavior is transport-independent

- **WHEN** the same authenticated MCP request is delivered over any supported transport
- **THEN** it is authenticated, normalized, and routed with identical outcomes

### Requirement: Loopback-only binding

In this version the face SHALL bind a local-only surface and SHALL NOT expose any remote or
non-loopback listener.

#### Scenario: The bound address is local-only

- **WHEN** the MCP face binds its transport
- **THEN** a network transport binds the loopback address only, and a socket transport binds
  a local socket reachable only from the same host

### Requirement: Per-session bearer authentication

Every MCP request SHALL be authenticated with the per-session bearer token, consistent with
the gate's other authenticated faces. An unauthenticated connection SHALL be refused before
the protocol loop serves it, and every routed request SHALL additionally carry the token
through the gate's shared authentication so an unauthenticated request never reaches a route.

#### Scenario: An authenticated request is served

- **WHEN** a client presents a valid per-session token and issues an MCP request
- **THEN** the request is authenticated and routed

#### Scenario: A connection with no valid token is refused at admission

- **WHEN** a client connects without a valid per-session token
- **THEN** the connection is refused before the MCP protocol loop serves any request

#### Scenario: A request with a wrong token is rejected before routing

- **WHEN** an MCP request carries a token that does not authenticate against the session
- **THEN** it is rejected as unauthenticated and never reaches a route

### Requirement: Normalization into an MCP inbound through the shared middleware

An accepted MCP request SHALL be normalized into an inbound of a distinct MCP kind and
routed through the gate's existing global middleware chain unchanged, so observation and
authentication apply to MCP exactly as to other inbound kinds. Each request SHALL be
observed exactly once and carry a correlation identifier.

#### Scenario: An MCP request flows through the global chain

- **WHEN** an authenticated MCP request is routed
- **THEN** it passes through the same global middleware as other inbound kinds and yields a
  terminal outcome (accepted or forwarded)

#### Scenario: Each request is observed once with a correlation id

- **WHEN** an MCP request is routed
- **THEN** exactly one observation record is emitted for it, carrying a correlation
  identifier that is consistent across the request's processing

### Requirement: Endpoint publication for discovery without secret disclosure

The gate SHALL publish the bound MCP endpoint to a discovery location in its base directory,
following the same convention as its other published endpoints, so an orchestrator can read
it and inject it into an agent CLI's MCP configuration. The per-session token SHALL NOT be
written to that discovery location.

#### Scenario: The endpoint is published after binding

- **WHEN** the MCP face has bound its transport
- **THEN** a discovery entry naming the reachable endpoint exists in the gate's base
  directory and is readable

#### Scenario: The discovery entry carries no token

- **WHEN** the discovery entry is read
- **THEN** it contains the endpoint location only and contains no per-session token

#### Scenario: The discovery entry is cleaned up on clean shutdown

- **WHEN** the gate shuts down cleanly
- **THEN** the MCP discovery entry is removed

### Requirement: Routing layer carries no tools in this version

This version provides the MCP routing layer only; it SHALL NOT register tool
implementations. A tool listing SHALL return an empty set, and the layer SHALL remain ready
for tool handlers to attach later without changes to authentication, normalization, or
routing.

#### Scenario: A tool listing returns an empty set

- **WHEN** an authenticated client requests the available tools
- **THEN** the face returns an empty tool set

### Requirement: Lifecycle teardown with the gate

The MCP face SHALL participate in the gate's graceful shutdown: on shutdown it SHALL stop
accepting new MCP connections and release its bound transport.

#### Scenario: The face stops on gate shutdown

- **WHEN** the gate begins graceful shutdown
- **THEN** the MCP face stops accepting new connections and releases its transport
