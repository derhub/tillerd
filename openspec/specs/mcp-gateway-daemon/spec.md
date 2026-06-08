# mcp-gateway-daemon Specification

## Purpose

The standalone daemon and its MCP front: manifest, detach, reuse-or-spawn, explicit stop, and the loopback streamable-http endpoint with bearer-token and origin authentication.

## Requirements

### Requirement: Standalone long-lived daemon

The gateway SHALL run as a standalone background process that continues running independently of any
client, including after a desktop UI that launched it has closed. Backends it has spawned SHALL
continue running while the daemon runs.

#### Scenario: Survives client exit

- **WHEN** the client that launched the daemon exits
- **THEN** the daemon and its backends SHALL keep running

#### Scenario: Detached from launcher

- **WHEN** the daemon is launched by another process
- **THEN** it SHALL detach from that process's session so it is not terminated when the launcher exits

### Requirement: Manifest and reuse-or-spawn

The daemon SHALL write a manifest at `mcp-gateway.json` in the application data directory recording
its process id, listening port, access token, and version. The manifest SHALL be written atomically
and removed on clean stop. A launcher SHALL read the manifest and connect to a running daemon whose
version matches, rather than starting a second instance; a stale manifest SHALL lead to a fresh
spawn.

#### Scenario: Manifest written on start

- **WHEN** the daemon starts and binds its port
- **THEN** it SHALL atomically write the manifest with its pid, port, token, and version

#### Scenario: Reuse running daemon

- **WHEN** a launcher finds a manifest whose process is alive and version matches
- **THEN** it SHALL connect to that daemon instead of spawning a new one

#### Scenario: Stale manifest respawns

- **WHEN** a launcher finds a manifest whose process is not alive
- **THEN** it SHALL spawn a fresh daemon and overwrite the manifest

#### Scenario: Manifest removed on clean stop

- **WHEN** the daemon stops cleanly
- **THEN** it SHALL remove the manifest

### Requirement: Explicit stop

The daemon SHALL stop only on explicit request or system shutdown, never as a side effect of a client
disconnecting.

#### Scenario: Client disconnect does not stop daemon

- **WHEN** all clients disconnect
- **THEN** the daemon SHALL keep running

### Requirement: MCP front over loopback

The daemon SHALL expose the aggregated MCP server over an HTTP streamable transport bound to the
loopback interface only. Any MCP client SHALL be able to connect using the standard transport with no
gateway-specific protocol.

#### Scenario: Bound to loopback

- **WHEN** the daemon starts its MCP endpoint
- **THEN** it SHALL bind only the loopback interface and SHALL NOT accept connections from other hosts

#### Scenario: Standard client connects

- **WHEN** a standard MCP client connects to the endpoint with a valid token
- **THEN** it SHALL complete the MCP handshake and use the aggregated server

### Requirement: Authenticated access

The daemon SHALL generate an access token at startup and SHALL require it as a bearer credential on
the MCP endpoint. It SHALL reject requests whose `Origin` is not a loopback origin. The token SHALL be
discoverable by a local client through the manifest.

#### Scenario: Missing or wrong token rejected

- **WHEN** a request to the MCP endpoint lacks the correct bearer token
- **THEN** the daemon SHALL reject it as unauthorized

#### Scenario: Non-loopback origin rejected

- **WHEN** a request carries an `Origin` that is not a loopback origin
- **THEN** the daemon SHALL reject it as forbidden

#### Scenario: Token available via manifest

- **WHEN** a local client reads the manifest
- **THEN** it SHALL obtain the token needed to authenticate
