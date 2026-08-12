## MODIFIED Requirements

### Requirement: Single aggregated MCP face

The gateway SHALL present itself to clients as one MCP server whose tools, resources, and prompts are the union of those exposed by all currently reachable backends together with authorized first-party tools. A client SHALL NOT need to know that multiple backends exist, and first-party tools SHALL use the existing standard MCP transport rather than a separate gateway face.

#### Scenario: Tool list is the union

- **WHEN** a client requests the tool list and two backends are reachable
- **THEN** the gateway SHALL return the combined tools of both backends together with authorized first-party tools

#### Scenario: Unreachable backend omitted

- **WHEN** a backend is not currently reachable
- **THEN** the gateway SHALL omit that backend's primitives from the aggregated listings without failing the request

#### Scenario: First-party tool is listed without a backend

- **WHEN** a client is authorized for a first-party view, review, or diff operation and no backend exposes that operation
- **THEN** the gateway lists the authorized first-party tool through the same MCP server
