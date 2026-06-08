## MODIFIED Requirements

### Requirement: Lifecycle-hook ingestion

The memory layer SHALL ingest session activity through lifecycle hooks covering session start,
user prompt submission, post-tool execution, agent stop, and session end. It SHALL consume the
**canonical hook-event contract** (already normalized from the raw agent format by the gate) and
SHALL NOT parse the raw agent payload itself. It SHALL obtain those events through a hook-source port
whose source is a subscription to the gate; the port SHALL keep ingestion testable with a stub
source. Hook delivery MUST be fire-and-forget and MUST NOT block the agent.

#### Scenario: Prompt captured on submission

- **WHEN** the user submits a prompt
- **THEN** the prompt content MUST be ingested as a chunk

#### Scenario: Tool execution captured

- **WHEN** the agent completes a tool execution that is not on the skip list
- **THEN** the tool name, input, and response MUST be ingested as a chunk

#### Scenario: Hook never blocks the agent

- **WHEN** a hook fires
- **THEN** the agent MUST proceed without waiting for storage to complete

#### Scenario: Source is swappable for tests

- **WHEN** hook events arrive from the gate subscription in production and from a stub source in tests
- **THEN** ingestion behavior MUST be identical regardless of the wired source

### Requirement: Out-of-band embedding on write

The write path SHALL persist a chunk synchronously and enqueue its embedding request onto a
durable queue that survives a restart. A background worker SHALL drain the queue proactively —
not lazily on read — so that neither capture latency nor recall latency depends on embedding
computation. Ingest SHALL be idempotent so that at-least-once draining cannot create duplicates.

#### Scenario: Chunk committed before embedding

- **WHEN** a chunk is ingested
- **THEN** the chunk MUST be committed to storage first
- **AND** an embedding request MUST be enqueued onto the durable queue

#### Scenario: Queue survives restart

- **WHEN** the memory tool restarts with embedding requests still pending
- **THEN** the pending requests MUST still be present and MUST be drained after restart

#### Scenario: Worker drains proactively

- **WHEN** embedding requests are pending and no recall is issued
- **THEN** the background worker MUST drain them without waiting for a read

#### Scenario: Idempotent ingest under at-least-once draining

- **WHEN** the same embedding request is processed more than once
- **THEN** no duplicate chunk or embedding MUST result
