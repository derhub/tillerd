# template-instance Specification

## Purpose
Snapshotting a template's spec blob and version onto a session row at creation time, written atomically with the session. Covers session-template divergence after instantiation -- template updates do not affect existing sessions and session spec edits do not affect the template -- and sessions created without a template reference, which carry null spec fields and produce no launches.
## Requirements
### Requirement: Template snapshot written to session on instantiation

When a session is created with a template reference, the store SHALL atomically read the
referenced template's spec blob and version, write both onto the new session row, and return
the created session. The template's blob is copied verbatim: no migration or re-parsing occurs
at copy time. If the referenced template does not exist, session creation SHALL return a typed
not-found error and no session row SHALL be written.

#### Scenario: Session carries spec snapshot after creation with template

- **WHEN** a session is created with a valid template reference
- **THEN** the session row carries the template's spec blob and version at creation time

#### Scenario: Missing template causes session creation to fail

- **WHEN** a session is created referencing a template that does not exist
- **THEN** session creation returns a typed not-found error and no session row is created

#### Scenario: Template and session write are atomic

- **WHEN** the store writes the session row with the spec snapshot
- **THEN** both the session row and the spec fields are written in the same transaction; a partial write does not occur

### Requirement: Session divergence after instantiation

After a session is created from a template, the session's spec SHALL be independent of the
template. Subsequent updates to the template SHALL NOT affect sessions that were already
instantiated from it. A session's spec MAY be updated without affecting the originating
template or any other session.

#### Scenario: Template update does not affect existing sessions

- **WHEN** a template's spec blob is updated after a session was instantiated from it
- **THEN** the session's spec blob remains unchanged

#### Scenario: Session spec update does not affect the template

- **WHEN** a session's spec blob is updated independently
- **THEN** the template's spec blob remains unchanged

### Requirement: Session without template reference carries no spec

A session created without a template reference SHALL have null spec blob and version fields
on its row. The executor SHALL treat a null spec as an empty spec: it performs no surface
launches unless the session spec is later set explicitly.

#### Scenario: No template means null spec fields

- **WHEN** a session is created without a template reference
- **THEN** the session row's spec blob and version are both null

#### Scenario: Null spec produces no launches

- **WHEN** the executor is invoked for a session whose spec blob is null
- **THEN** no launch items are executed and no error is reported

