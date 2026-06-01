# agent-content

## Purpose

Defines how the engine reads structured content from an agent's session transcript. Content is read on lifecycle events (PostToolUse, Stop) and on process exit, never via file watching or polling.

## Requirements

### Requirement: Read-on-hook transcript reading

The content reader SHALL read new entries from the agent's session transcript triggered by lifecycle events — on PostToolUse and on Stop — and SHALL also do a final read on process exit. It SHALL NOT use a file watcher or a polling timer.

#### Scenario: Read delta on tool completion

- **WHEN** a PostToolUse `HookEvent` is dispatched for a session
- **THEN** the content reader SHALL read the transcript entries appended since its last read and emit content from them

#### Scenario: Final read on exit

- **WHEN** the session process exits
- **THEN** the content reader SHALL perform a final read so end-of-session content is captured even if no further hooks arrive

### Requirement: Delta tracking by byte offset with reset on rewrite

The content reader SHALL track a byte offset so each read returns only new entries, and SHALL reset and re-read from the start if the file shrinks below the offset or its identity changes (e.g. the transcript is rewritten or compacted).

#### Scenario: Incremental reads

- **WHEN** the transcript grows between two reads
- **THEN** the reader SHALL emit content only from the newly appended entries

#### Scenario: Transcript rewritten

- **WHEN** the transcript file is truncated or replaced so its size is below the tracked offset
- **THEN** the reader SHALL reset the offset and re-read from the start rather than read garbage

### Requirement: Typed content events

The content reader SHALL emit typed events for tool use, file edits, and usage/cost, derived by calling the adapter's transcript parse function, using a shape independent of the transport that produced the session.

#### Scenario: Tool use event

- **WHEN** a transcript entry describes a tool invocation
- **THEN** the reader SHALL emit a typed tool-use content event with the tool name and input

#### Scenario: Edit event

- **WHEN** a transcript entry describes a file edit
- **THEN** the reader SHALL emit a typed edit content event identifying the file and change

#### Scenario: Usage event

- **WHEN** a transcript entry contains token usage or cost
- **THEN** the reader SHALL emit a typed usage content event with token counts and cost when present

### Requirement: Graceful absence

A missing or not-yet-written transcript SHALL be treated as empty content and surfaced as a `TranscriptUnavailable` typed error rather than crashing the session; the drive and status planes SHALL continue.

#### Scenario: Transcript not yet present

- **WHEN** a read is triggered before the transcript file exists
- **THEN** the reader SHALL treat it as empty, emit no content, and not error the session
