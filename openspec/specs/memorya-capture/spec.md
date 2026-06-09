# memorya-capture

## Purpose

Defines how the memory layer ingests session activity through lifecycle hooks: capturing prompts and tool events as chunks, suppressing low-value and duplicate events, titling tool events, requesting embeddings out-of-band, and indexing project documentation.
## Requirements
### Requirement: Lifecycle-hook ingestion

The memory layer SHALL ingest session activity through lifecycle hooks covering session start, user prompt submission, post-tool execution, agent stop, and session end. Hook delivery MUST be fire-and-forget and MUST NOT block the agent.

#### Scenario: Prompt captured on submission

- **WHEN** the user submits a prompt
- **THEN** the prompt content MUST be ingested as a chunk

#### Scenario: Tool execution captured

- **WHEN** the agent completes a tool execution that is not on the skip list
- **THEN** the tool name, input, and response MUST be ingested as a chunk

#### Scenario: Hook never blocks the agent

- **WHEN** a hook fires
- **THEN** the agent MUST proceed without waiting for storage to complete

### Requirement: Skip list for low-value events

The memory layer SHALL suppress ingestion of low-value tool events (such as infrastructure listing, command invocation, skill invocation, task-list management, and user-interaction prompts) so the store is not polluted with noise.

#### Scenario: Skip-listed tool ignored

- **WHEN** a post-tool event names a tool on the skip list
- **THEN** no chunk MUST be created for that event

### Requirement: Tool-event titling

The memory layer SHALL derive a concise title for tool-event chunks from the tool name and its primary argument, so results are scannable without expanding full content.

#### Scenario: Title derived from tool and argument

- **WHEN** a tool-event chunk is created
- **THEN** a title MUST be derived from the tool name and its primary argument

### Requirement: Duplicate-fire suppression

The memory layer SHALL suppress duplicate ingestion when the same lifecycle event fires more than once within a session, without dropping genuinely distinct events that happen to share content across sessions.

#### Scenario: Repeated hook fire deduplicated

- **WHEN** the same event (same session, position, and kind) is ingested more than once
- **THEN** only one chunk MUST be retained

#### Scenario: Identical content in different sessions retained

- **WHEN** two distinct sessions ingest chunks with identical content
- **THEN** both chunks MUST be retained as separate records

### Requirement: Out-of-band embedding on write

The write path SHALL persist a chunk synchronously and request its embedding out-of-band, so capture latency is bounded and never depends on embedding computation.

#### Scenario: Chunk committed before embedding

- **WHEN** a chunk is ingested
- **THEN** the chunk MUST be committed to storage first
- **AND** an embedding request MUST be enqueued for asynchronous processing

### Requirement: Project document indexing

On session start the memory layer SHALL index the project's documentation files into searchable doc chunks, replacing any prior doc chunks for that project, so the agent can answer questions about a project regardless of what it has read this session. Indexing MUST run without blocking the session.

#### Scenario: Documents chunked by heading

- **WHEN** the project is indexed
- **THEN** each documentation file MUST be split into chunks at heading boundaries
- **AND** each chunk MUST be stored as a document chunk with its source file path and an embedding request

#### Scenario: Stale document chunks replaced

- **WHEN** the project is re-indexed on a later session start
- **THEN** the project's previously stored document chunks MUST be removed before the fresh chunks are stored

#### Scenario: Excluded directories skipped

- **WHEN** indexing scans the project
- **THEN** dependency, build-output, and version-control directories MUST be excluded

### Requirement: Capture-time redaction

The memory layer SHALL redact captured content through the redaction library before any
write, so that credentials and structured PII are not persisted in a stored chunk, title,
digest, fact, or entity. Redaction MUST be applied to captured prompt content and to
post-tool capture, covering both the tool response body and the tool-input-derived title.
Project documents remain exempt and are indexed verbatim. Downstream digests, facts, and
entities MUST derive only from already-redacted stored chunks and MUST NOT read raw hook
input.

#### Scenario: Secret in a prompt redacted before storage

- **WHEN** a captured prompt contains a secret
- **THEN** the stored chunk MUST contain `[REDACTED]` in place of the secret
- **AND** the original secret MUST NOT appear in any stored chunk, title, digest, or fact

#### Scenario: Secret in a tool response redacted before storage

- **WHEN** a captured tool response contains a secret or structured PII value
- **THEN** the stored chunk MUST contain `[REDACTED]` in place of that value

#### Scenario: Secret in a tool-input-derived title redacted

- **WHEN** a captured tool event has an input whose auto-derived title contains a secret
- **THEN** the secret MUST be redacted in both the stored title field and the composed chunk content

#### Scenario: Documents indexed verbatim

- **WHEN** a project document is indexed
- **THEN** redaction MUST NOT alter it and its content MUST be stored verbatim

