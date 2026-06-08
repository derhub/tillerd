## ADDED Requirements

### Requirement: Middleware router

The gate SHALL route every inbound through composable middleware over a context, yielding an outbound
or a typed rejection. Middleware SHALL be composable both sequentially and concurrently, and SHALL be
declared at the composition root, not hard-coded in the gate. The router SHALL dispatch an inbound to
global middleware plus the middleware bound to the inbound's kind. In v1 the global middleware SHALL
be **observe** (outermost) and **authenticate**, ordered so observe wraps authenticate and records
its rejections; further middleware (validate, firewall, redaction) SHALL be addable to a route later
without changing the gate.

#### Scenario: Inbound runs globals then its route

- **WHEN** an inbound is handled
- **THEN** the router SHALL run the global middleware and then the middleware for the inbound's kind, in declared order

#### Scenario: A middleware short-circuits

- **WHEN** a middleware returns a rejection
- **THEN** the router SHALL stop and return that typed rejection, running no later middleware

#### Scenario: Sequential and concurrent composition

- **WHEN** middleware are composed
- **THEN** the composition SHALL support both sequential ordering and concurrent (joined) execution

### Requirement: Tool routing without a tool protocol

The gate SHALL accept tool inbounds (tool call and tool result) on its tool route and return an
outbound to the caller; the caller owns its transport and routing. The gate SHALL NOT speak any
tool-call protocol and SHALL hold no backend registry. In v1 the tool route runs the globals and
passes the body through.

#### Scenario: Caller routes a tool inbound and uses the outbound

- **WHEN** a caller sends a tool inbound to the gate
- **THEN** the gate SHALL return an outbound (or a typed rejection), and the caller SHALL route the result itself

#### Scenario: Gate carries no tool-call knowledge

- **WHEN** the gate's surface is enumerated
- **THEN** it SHALL contain no tool-call protocol and no backend registry

### Requirement: Observability of every inbound

The gate SHALL emit a structured observability record for every inbound it handles (hooks and tool
inbounds), bound with the session id and correlation id and stamped with a gate resource identity.

#### Scenario: Inbound produces a correlation-bound record

- **WHEN** the gate handles any inbound
- **THEN** it SHALL emit a structured record carrying at least the session id, correlation id, kind, and outcome

### Requirement: Native agent-hook flow

The gate SHALL host the agent-hook flow on its hook route: it SHALL accept fire-and-forget hook
callbacks on a loopback endpoint, run the global middleware, normalize the hook to the canonical
hook-event contract via the injected adapter, and fan out the canonical event — carrying its session
id and correlation id — to consumers subscribed by session id. The session id used for routing SHALL
come from the authenticated token. Delivery to this endpoint SHALL be possible with a simple command,
requiring no dedicated receiver process.

#### Scenario: Hook posted, normalized, fanned out

- **WHEN** an agent hook callback is posted to the gate's hook endpoint
- **THEN** the gate SHALL run the globals, normalize it to the canonical hook event via the injected adapter, and deliver that event to every consumer subscribed to that session

#### Scenario: Hook delivery is fire-and-forget

- **WHEN** a hook is posted
- **THEN** the poster SHALL NOT need to wait for routing or any consumer

#### Scenario: Multiple consumers receive the same hook

- **WHEN** more than one consumer is subscribed to a session
- **THEN** each subscribed consumer SHALL receive the canonical hook event

### Requirement: Agent-specific normalization via an injected adapter

Normalizing a hook into the canonical hook-event contract SHALL be performed by an injected agent
adapter. The gate SHALL depend only on the adapter interface and carry no concrete agent parsing. The
canonical hook event SHALL carry typed fields sufficient for status and capture, so no consumer needs
the raw agent format.

#### Scenario: Normalization uses the injected adapter

- **WHEN** a hook is normalized
- **THEN** the gate SHALL produce the canonical hook event by calling the injected adapter, not by hard-coded per-agent logic

#### Scenario: Gate is agent-agnostic in code

- **WHEN** the gate is used with a different agent
- **THEN** only the injected adapter SHALL change, with no change to the gate itself

### Requirement: Hook-event subscription

The gate SHALL expose a subscription through which a consumer registers interest in a session's
hook events and receives them as they are produced, and through which it stops receiving them when
the session ends or the consumer deregisters. The subscription wire SHALL be versioned and mirrored
across the language contract surfaces so consumers in either runtime observe the same shape.

#### Scenario: Consumer receives a session's events

- **WHEN** a consumer is subscribed to a session and a hook event is produced for it
- **THEN** the consumer SHALL receive that event

#### Scenario: Subscription ends on session end

- **WHEN** a session ends
- **THEN** the gate SHALL stop delivering its events and SHALL release the subscription

### Requirement: Bounded delivery with a drop policy

Per-session delivery SHALL be bounded and SHALL NOT grow without limit or block ingestion. When a
consumer cannot keep up, the gate SHALL drop the oldest pending events for that consumer, count the
drops, and log them, rather than blocking the fire-and-forget poster.

#### Scenario: Slow consumer does not block ingestion

- **WHEN** a subscribed consumer falls behind and its buffer is full
- **THEN** the gate SHALL drop the oldest pending events, record a drop count, and continue accepting new hooks without blocking the poster

### Requirement: Correlation id on every message

Every message the gate handles SHALL carry a correlation id identifying one logical action across
process hops. The gate SHALL accept a caller-supplied correlation id and SHALL assign one when
absent; it SHALL preserve that id through the middleware, normalization, and fan-out, place it on every
emitted hook event, and bind it together with the session id on every log record it writes.

#### Scenario: Supplied correlation id is preserved

- **WHEN** a caller supplies a correlation id
- **THEN** the gate SHALL carry that id through to the emitted event and its log records

#### Scenario: Missing correlation id is assigned

- **WHEN** a message arrives without a correlation id
- **THEN** the gate SHALL assign one and use it consistently for that message's processing

### Requirement: Face isolation

The gate's hook endpoint, tool route, and administration surface SHALL be isolated. A caller of the
tool route SHALL NOT be able to inject hook events into the fan-out, and neither the hook endpoint nor
the tool route SHALL be able to perform administration; administration SHALL live on a separate
authenticated surface.

#### Scenario: Tool caller cannot inject hooks

- **WHEN** a caller uses the tool route
- **THEN** it SHALL NOT be able to cause a hook event to be fanned out to subscribers

#### Scenario: Agent surfaces cannot administer

- **WHEN** a caller on the hook endpoint or the tool route attempts an administrative action
- **THEN** the gate SHALL reject it and require the separate administration surface

### Requirement: Loopback binding and token

Every gate endpoint SHALL bind the loopback interface only. Non-health endpoints SHALL require a
token; the health endpoint SHALL be unauthenticated and report only liveness and version.

#### Scenario: Endpoints bind loopback only

- **WHEN** the gate starts
- **THEN** each endpoint SHALL bind the loopback interface and SHALL NOT be reachable off-host

#### Scenario: Token required except for health

- **WHEN** a request reaches a non-health endpoint without a valid token
- **THEN** the gate SHALL reject it
- **AND** a health request SHALL be answered without a token
