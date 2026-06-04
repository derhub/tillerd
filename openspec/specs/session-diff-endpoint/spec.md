# session-diff-endpoint

## Purpose

Defines the server HTTP endpoint that returns the unified diff of a session's working directory, so a UI can display the changes an agent has produced.

## Requirements

### Requirement: Session diff retrieval endpoint

The server SHALL expose `GET /api/sessions/:id/diff` that returns the unified diff of the session's working directory. The session's `cwd` SHALL be resolved from persistent storage using the session ID. The response SHALL be plain text (`text/plain`) containing the full unified patch output.

#### Scenario: Successful diff

- **WHEN** a GET request is made to `/api/sessions/:id/diff` for an existing session
- **THEN** the server resolves the session's working directory, runs a git diff, and returns the unified patch as plain text with status 200

#### Scenario: Session not found

- **WHEN** a GET request is made for a session ID that does not exist in storage
- **THEN** the server returns 404 with a JSON error body `{ "error": "session not found" }`

#### Scenario: Not a git repository

- **WHEN** the session's working directory is not a git repository
- **THEN** the server returns 200 with an empty body (no patch output)

#### Scenario: No changes

- **WHEN** the git diff produces no output (clean working tree)
- **THEN** the server returns 200 with an empty body

### Requirement: CORS headers on diff endpoint

The diff endpoint SHALL include the same CORS headers as other HTTP endpoints in the server, permitting cross-origin requests from the development UI origin.

#### Scenario: CORS preflight

- **WHEN** an OPTIONS request is issued to `/api/sessions/:id/diff`
- **THEN** the server responds with appropriate CORS headers and status 204
