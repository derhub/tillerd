## ADDED Requirements

### Requirement: Consumer-oblivious subscription

The PTY daemon SHALL serve any subscriber by session id without carrying knowledge of the
subscriber's identity or purpose. Adding, changing, or removing a downstream consumer SHALL
require no change to the daemon.

#### Scenario: Daemon serves an unknown subscriber

- **WHEN** a client subscribes to a session by id
- **THEN** the daemon SHALL serve it without knowing what the consumer is or does

#### Scenario: New consumer needs no daemon change

- **WHEN** a new kind of consumer begins subscribing
- **THEN** the daemon SHALL require no modification to support it

### Requirement: Session-event subscription surface

The daemon's public surface SHALL be a session-event subscription: subscribers receive a session's
output stream and lifecycle events (such as exit) keyed by session id. This subscription SHALL be
the contracted way consumers observe sessions.

#### Scenario: Subscriber receives output and lifecycle

- **WHEN** a client subscribes to a live session
- **THEN** it SHALL receive that session's output stream and its lifecycle events keyed by session id

### Requirement: Versioned, mirrored wire

The subscription wire SHALL be versioned, and the version set SHALL be negotiated so client and
daemon select a mutually supported version. The wire's shape SHALL be mirrored across the
language contract surfaces from a single source of truth so the surfaces cannot drift.

#### Scenario: Version negotiated on connect

- **WHEN** a client connects advertising supported versions
- **THEN** the daemon SHALL select the highest mutually supported version

#### Scenario: Single source of truth for the wire

- **WHEN** the wire shape changes
- **THEN** the change SHALL originate from one versioned definition mirrored to every language surface

### Requirement: No hook ingress on the daemon surface

The daemon's public surface SHALL NOT include hook ingress. Agent hook input SHALL be owned by the
gate, not the daemon.

#### Scenario: Daemon exposes no hook face

- **WHEN** the daemon's public surface is enumerated
- **THEN** it SHALL contain no hook ingress face
