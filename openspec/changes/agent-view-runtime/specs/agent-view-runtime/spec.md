## ADDED Requirements

### Requirement: Isolated agent view document

The system SHALL render agent-supplied HTML or host-rendered Markdown only in a right-dock iframe with an opaque origin and `sandbox="allow-scripts"`. The iframe SHALL NOT receive same-origin, navigation, popup, form, download, pointer-lock, top-level, desktop-runtime, filesystem, process, or network authority. The system SHALL apply a strict per-view content policy that permits only the bundled view runtime, bundled rendering styles, and explicitly supplied inline images; it SHALL deny connections, nested frames, workers, media, fonts, forms, and navigation. The outer desktop content policy SHALL remain a separate boundary and SHALL NOT relax the iframe policy.

#### Scenario: Rendered content attempts direct access

- **WHEN** agent-supplied view content attempts a network, desktop-runtime, filesystem, or navigation operation
- **THEN** the isolated document does not receive that authority

#### Scenario: Host renders Markdown

- **WHEN** an agent publishes Markdown content for a view
- **THEN** the host renders it into the same isolated document without granting the renderer access to the app document

### Requirement: Capability-gated bridge lifecycle

The system SHALL create one bounded bridge endpoint per live view instance. Every request SHALL identify the view instance, a monotonically increasing request identifier, and a declared capability; the host SHALL accept it only from that instance's iframe window while that instance is active. The host SHALL reject any request identifier that is not strictly greater than the last accepted identifier for that instance, without executing the requested capability. The host SHALL validate each payload, enforce size and rate limits, and return a typed success or error response. A bridge endpoint SHALL expose only individually granted capabilities and SHALL NOT mint capabilities, access another session, invoke management operations, or expose a bearer token or generic MCP client. Closing or replacing a view SHALL revoke its endpoint and reject outstanding requests.

#### Scenario: View requests an ungranted capability

- **WHEN** a live view sends a bridge request for a capability that the host did not grant to that instance
- **THEN** the host rejects the request with a typed authorization error

#### Scenario: View replays a request identifier

- **WHEN** a live view sends a request identifier equal to or lower than its last accepted identifier
- **THEN** the host rejects the request with a typed invalid-request error and does not execute the capability

#### Scenario: View closes with pending work

- **WHEN** a view closes or is replaced while bridge requests are pending
- **THEN** the system revokes the endpoint and rejects every outstanding request
