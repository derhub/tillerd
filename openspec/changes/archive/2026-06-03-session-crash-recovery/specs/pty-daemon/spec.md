## ADDED Requirements

### Requirement: Durable stopped-session set

The daemon SHALL persist stopped-session identifiers to the durable session-persistence store so that a stopped session remains ineligible for resume across engine, server, and daemon restarts. The daemon SHALL consult the durable record when evaluating a resume request. Any in-memory set SHALL be a bounded cache over the durable record.

#### Scenario: Stop recorded durably

- **WHEN** the daemon receives a stop command for a session
- **THEN** the daemon SHALL terminate the session and record its session id in the durable stopped-session store

#### Scenario: Stop survives daemon restart

- **WHEN** a session is stopped, the daemon is restarted, and a resume is later requested for that session id
- **THEN** the daemon SHALL reject the resume with a `SessionStopped` typed error, having consulted the durable store

#### Scenario: Bounded cache does not resurrect resumability

- **WHEN** the in-memory stopped-session cache evicts an entry that is still recorded in the durable store
- **THEN** a resume request for that session id SHALL still be rejected, because the durable record is authoritative

### Requirement: sessionId re-registration after eviction

The daemon registry SHALL accept re-registration of a session id it recently evicted on exit, so a crashed session can be recovered under the same id.

#### Scenario: Re-register an evicted session id

- **WHEN** a session exits and is evicted, then a spawn with that same session id arrives for recovery
- **THEN** the daemon SHALL accept the registration and manage the new process under that session id
