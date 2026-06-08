## MODIFIED Requirements

### Requirement: Agent-facing and human-facing surfaces

Recall SHALL be exposed to the agent over a tool interface and to a human over a viewer bound to
the loopback interface only. The agent-facing tool interface SHALL be dual-mode: a standalone tool
face when the memory tool runs alone, or fronted by the tool gateway as an ordinary backend when
composed — with identical tool behavior in either mode and no special-casing by the gateway. The
human viewer SHALL remain loopback-only in both modes.

#### Scenario: Viewer is loopback-only

- **WHEN** the human viewer is served
- **THEN** it MUST bind to the loopback interface only

#### Scenario: Standalone exposes its own tool face

- **WHEN** the memory tool runs alone
- **THEN** recall MUST be reachable over its own standalone tool face

#### Scenario: Composed is fronted by the tool gateway

- **WHEN** the memory tool is composed with the tool gateway
- **THEN** recall MUST be reachable as a backend behind the gateway's single tool front
- **AND** the gateway MUST treat it like any other backend with no special-casing
