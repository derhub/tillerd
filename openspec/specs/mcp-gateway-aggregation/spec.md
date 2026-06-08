# mcp-gateway-aggregation Specification

## Purpose

Presenting many backends as one MCP server: tool/resource/prompt namespacing and routing, capability union, the in-memory registry, the reverse-proxy relay, and notification forwarding.

## Requirements

### Requirement: Single aggregated MCP face

The gateway SHALL present itself to clients as one MCP server whose tools, resources, and prompts
are the union of those exposed by all currently reachable backends. A client SHALL NOT need to know
that multiple backends exist.

#### Scenario: Tool list is the union

- **WHEN** a client requests the tool list and two backends are reachable
- **THEN** the gateway SHALL return the combined tools of both backends

#### Scenario: Unreachable backend omitted

- **WHEN** a backend is not currently reachable
- **THEN** the gateway SHALL omit that backend's primitives from the aggregated listings without
  failing the request

### Requirement: Namespacing and routing

The gateway SHALL prefix each backend's tool and prompt names with the backend name and a reserved
separator so that identically named tools or prompts from different backends do not collide. On
invocation the gateway SHALL strip the prefix and route the request to the owning backend. Resources
keep their original URI (URIs are globally unique by convention) and are routed by a `uri -> backend`
owner map; if two backends expose the same URI, the most recently indexed backend owns it. A request
naming a primitive with no known owner SHALL fail with a typed error.

#### Scenario: Names are prefixed

- **WHEN** a backend named `github` exposes a tool `create_issue`
- **THEN** the aggregated tool SHALL be named `github` followed by the separator and `create_issue`

#### Scenario: Call routes to owner

- **WHEN** a client calls a namespaced tool
- **THEN** the gateway SHALL strip the namespace and invoke the tool on the owning backend, returning
  its result

#### Scenario: Unknown primitive rejected

- **WHEN** a client calls a tool whose namespaced name has no owning backend
- **THEN** the gateway SHALL return a typed error

#### Scenario: Separator inside an original name preserved

- **WHEN** a backend tool name itself contains the separator
- **THEN** the gateway SHALL split only on the first separator so the original name is reconstructed
  correctly

#### Scenario: Resource routed by its URI

- **WHEN** a client reads a resource by URI
- **THEN** the gateway SHALL route the read to the backend recorded as that URI's owner

#### Scenario: Duplicate resource URI is last-wins

- **WHEN** two backends expose the same resource URI
- **THEN** the most recently indexed backend SHALL own that URI

### Requirement: Capability union

The gateway SHALL advertise only the capabilities that at least one reachable backend supports. It
SHALL NOT advertise a primitive category that no backend provides.

#### Scenario: Advertise present categories

- **WHEN** at least one backend provides prompts
- **THEN** the gateway SHALL advertise the prompts capability

#### Scenario: Omit absent categories

- **WHEN** no backend provides resources
- **THEN** the gateway SHALL NOT advertise the resources capability

### Requirement: Registry with generation invalidation and allowlist filtering

The gateway SHALL maintain an in-memory index mapping each namespaced primitive to its owning
backend, with a generation counter that advances whenever any backend's primitive set changes. The
index SHALL apply each backend's allowlist when admitting its tools. The gateway SHALL NOT persist
the index to disk.

#### Scenario: Generation advances on change

- **WHEN** a backend's tool set changes
- **THEN** the registry generation SHALL advance

#### Scenario: Allowlist applied at index time

- **WHEN** a backend declares an allowlist
- **THEN** only allowed tools SHALL be admitted to the index for that backend

#### Scenario: Downed backend dropped

- **WHEN** a backend becomes unreachable
- **THEN** its entries SHALL be removed from the index and the generation SHALL advance

### Requirement: Reverse-direction request relay

The gateway SHALL relay server-to-client requests originated by a backend to the connected front
client, and return the client's response to the originating backend. This SHALL include model
sampling requests, roots listing, and elicitation requests. The gateway SHALL forward the front
client's roots and roots-changed notifications to backends that declare the roots capability.

#### Scenario: Sampling relayed to client

- **WHEN** a backend issues a model sampling request
- **THEN** the gateway SHALL forward it to the front client and return the client's result to the
  backend

#### Scenario: Elicitation relayed to client

- **WHEN** a backend issues an elicitation request
- **THEN** the gateway SHALL forward it to the front client and return the client's response to the
  backend

#### Scenario: Roots forwarded to backends

- **WHEN** the front client provides or changes its roots
- **THEN** the gateway SHALL make the current roots available to backends that support roots

### Requirement: Notification forwarding

The gateway SHALL forward a backend's list-changed notifications (for tools, resources, and prompts)
to the front client after re-indexing the affected backend, and SHALL forward a client's
cancellation notification to the backend handling the cancelled request.

#### Scenario: List-changed propagated

- **WHEN** a backend signals that its tools changed
- **THEN** the gateway SHALL re-index that backend and notify the front client that the tool list
  changed

#### Scenario: Cancellation propagated

- **WHEN** the front client cancels an in-flight request
- **THEN** the gateway SHALL forward the cancellation to the backend handling that request

### Requirement: Backend-tagged errors

When a backend returns an error or is unavailable, the gateway SHALL surface a typed error to the
client that identifies which backend produced or could not service the request.

#### Scenario: Error names the backend

- **WHEN** a routed call fails inside a backend
- **THEN** the gateway SHALL return an error to the client that names the backend
