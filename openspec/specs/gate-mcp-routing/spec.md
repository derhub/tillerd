# gate-mcp-routing Specification

## Purpose

The gate's MCP face: the `Mcp` route of the gate's single front-door socket, peer to hook/tool/subscribe/admin. A connection opens with the gate's route preamble; once its per-session token is verified the stream upgrades to the MCP protocol. The face normalizes each request into a distinct MCP inbound kind and routes it through the gate's existing global middleware (observe, auth) unchanged. The path is derived from the runtime directory — there is no separate transport and no published endpoint. This version is the routing layer only — it registers no tool implementations.

## Requirements

### Requirement: MCP ingress face

The gate SHALL expose an MCP ingress face as the `Mcp` route of its single front-door
socket, peer to its other routes. A connection SHALL open with the gate's route preamble
selecting the `Mcp` route; once the preamble's session token is verified, the gate SHALL
upgrade the connection and speak the MCP protocol — including the initialize handshake and
protocol-version negotiation — over a maintained protocol library rather than a hand-rolled
wire, so the gate owns only the admission preamble and the bridge from an MCP request to an
internal inbound.

#### Scenario: A compliant client completes the initialize handshake

- **WHEN** a client connects on the `Mcp` route with a verified preamble and sends an
  initialize request with its supported protocol version
- **THEN** the face completes the handshake, negotiating a mutually supported protocol
  version, and the connection becomes ready to serve requests

#### Scenario: Version negotiation fails for an unsupported client

- **WHEN** an MCP client requests a protocol version the face cannot satisfy
- **THEN** the face declines the handshake with a protocol error and does not serve
  requests on that connection

### Requirement: Loopback-only binding

The MCP face SHALL NOT bind a transport of its own. It SHALL be served as the `Mcp` route of
the gate's single local Unix domain socket, reachable only from the same host, and SHALL NOT
expose any remote or non-loopback listener.

#### Scenario: The bound surface is local-only

- **WHEN** the gate binds its socket
- **THEN** the MCP face is reachable only as the `Mcp` route of that local socket, with no
  remote or network listener of its own

### Requirement: Per-session bearer authentication

Every MCP connection SHALL be admitted only with a valid per-session token carried in its
route preamble, consistent with the gate's other session-token routes. A connection whose
preamble token does not verify SHALL be refused before the connection is upgraded to the MCP
protocol, and every routed request SHALL additionally carry the token through the gate's
shared authentication so an unauthenticated request never reaches a route.

#### Scenario: An authenticated connection is served

- **WHEN** a client presents a valid per-session token in its `Mcp`-route preamble and
  issues an MCP request
- **THEN** the connection is upgraded, and the request is authenticated and routed

#### Scenario: A connection with no valid token is refused before upgrade

- **WHEN** a client connects on the `Mcp` route without a valid per-session token in its
  preamble
- **THEN** the connection is refused before it is upgraded to the MCP protocol

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

### Requirement: Routing layer carries no tools in this version

This version provides the MCP routing layer only; it SHALL NOT register tool
implementations. A tool listing SHALL return an empty set, and the layer SHALL remain ready
for tool handlers to attach later without changes to authentication, normalization, or
routing.

#### Scenario: A tool listing returns an empty set

- **WHEN** an authenticated client requests the available tools
- **THEN** the face returns an empty tool set

### Requirement: Lifecycle teardown with the gate

The MCP face SHALL participate in the gate's graceful shutdown through the single listener:
on shutdown the gate SHALL stop accepting new connections on its socket, so the `Mcp` route
stops serving without a face-specific transport to release.

#### Scenario: The face stops on gate shutdown

- **WHEN** the gate begins graceful shutdown
- **THEN** the `Mcp` route stops accepting new connections together with every other route
