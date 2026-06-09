## ADDED Requirements

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
