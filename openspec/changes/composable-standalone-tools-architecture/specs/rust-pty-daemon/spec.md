## ADDED Requirements

### Requirement: Consumer-oblivious operation

The daemon SHALL implement the session-subscription contract and SHALL carry no knowledge of any
downstream consumer. It SHALL serve subscribers by session id and SHALL require no change when a
consumer is added, changed, or removed.

#### Scenario: Daemon unaffected by consumer changes

- **WHEN** a downstream consumer is added or removed
- **THEN** the daemon SHALL require no modification and SHALL continue serving subscribers by session id

## REMOVED Requirements

### Requirement: Optional hook ingress capability

**Reason**: Hook ingress is consolidated into the ingress gate, which becomes the single
agent-facing trust boundary for all untrusted input. Keeping a second hook ingress in the daemon
duplicates the trust surface and couples a second protocol to the PTY core.

**Migration**: Agent hooks SHALL be delivered to the gate's hook face instead of the daemon's hook
socket. Consumers that previously relied on the daemon relaying raw hook payloads SHALL instead
receive normalized hook events by subscribing to the gate. The cli's hook installer SHALL point the
installed hook at the gate (the universal ingress) rather than the daemon.
