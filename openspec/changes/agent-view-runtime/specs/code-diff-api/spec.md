## ADDED Requirements

### Requirement: Project-rooted bounded diff retrieval

The system SHALL compute diffs only for the repository rooted at an existing project record. A diff request SHALL name exactly one project and one bounded target: working tree, staged changes, or an explicit commit range. The system SHALL reject a project-relative path outside that root and SHALL return typed errors for a missing repository, invalid revision, unavailable target, cancelled request, or oversized output. The diff API SHALL be read-only and SHALL NOT expose filesystem handles, shell commands, or arbitrary file access.

#### Scenario: Path escapes the project root

- **WHEN** a request names a project-relative path that resolves outside the project's repository root
- **THEN** the system rejects the request with a typed invalid-path error

#### Scenario: Explicit commit range is valid

- **WHEN** a request names an existing project and a valid bounded commit range
- **THEN** the system returns only the diff for that project and range

### Requirement: Structured diff model

The diff API SHALL return a structured read-only model containing changed files, hunks, and line mappings. Consumers SHALL render this model without receiving repository authority. The system SHALL preserve target identity with the result so a consumer can detect a stale target before an operation that depends on it.

#### Scenario: UI receives a diff result

- **WHEN** the diff API returns changes for a valid target
- **THEN** the UI receives files, hunks, line mappings, and the target identity without a filesystem handle or command
