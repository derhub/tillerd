## MODIFIED Requirements

### Requirement: Forward the lifecycle payload to the loopback receiver

The hook callback client SHALL forward the agent's lifecycle payload to the gate's single socket on the `Hook` route. It SHALL open the connection with the gate's route preamble — selecting the `Hook` route and carrying the session id and session token — and then write the lifecycle payload as a length-prefixed frame using the gate's shared frame codec.

#### Scenario: Forward over the local socket

- **GIVEN** the agent invokes the client with a lifecycle payload on standard input, and the runtime directory, session id, and session token provided via the environment
- **WHEN** the client runs
- **THEN** it SHALL derive the gate socket path from the runtime directory, open the connection with a route preamble selecting the `Hook` route and carrying the session id and session token, and write the payload frame to the socket
- **AND** it SHALL carry the session id and session token inside the route preamble, not as transport headers

#### Scenario: Gate socket absent

- **GIVEN** the gate socket is absent or unreachable
- **WHEN** the client runs
- **THEN** it SHALL exit without error and forward nothing
