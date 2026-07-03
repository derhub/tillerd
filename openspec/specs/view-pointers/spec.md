# view-pointers

## Purpose

Durable UI position pointers — the active workspace, the last-visited session per
project, and per-project sidebar expansion — stored in the orchestrator settings store
and wired through the client's server-state cache with optimistic local reads.

## Requirements

### Requirement: View pointers persist in the settings store

The view pointers `activeWorkspace`, `lastSession.<project>`, and
`sidebar.expanded.<project>` SHALL persist as keyed values in the orchestrator settings
store (global scope). They SHALL NOT persist in webview browser storage. Writes SHALL be
fire-and-forget from the user's perspective: pointer updates never block or fail the
interaction that caused them.

#### Scenario: Pointers survive a webview storage wipe

- **WHEN** the webview's browser storage is cleared and the app relaunches
- **THEN** the active workspace, per-project last session, and sidebar expansion restore
  from the settings store

#### Scenario: Pointer write failure does not break the interaction

- **WHEN** persisting a pointer fails (e.g. transient store error)
- **THEN** the user-visible interaction (switching workspace, expanding a project)
  completes normally and the failure is reported only through the standard error channel

### Requirement: Pointers read through the server-state cache with optimistic updates

Pointer reads SHALL flow through the client's server-state cache under dedicated query
keys; pointer writes SHALL update the local value optimistically (instant UI) and settle
through the standard mutation-invalidation path so sibling windows converge.

#### Scenario: Workspace switch is instant and coherent

- **WHEN** the user switches the active workspace
- **THEN** the sidebar re-scopes immediately in that window, and other open windows
  observe the updated pointer through the standard invalidation broadcast

### Requirement: Pointers resolve against live lifecycle

A pointer SHALL be resolved against the current lifecycle state of its target before
use: a pointer to an archived or deleted workspace resolves to the Default workspace; a
`lastSession` pointer to an archived or deleted session is ignored (the project opens
with no session preselected). Stale pointers SHALL never produce an error or an empty
shell.

#### Scenario: Active-workspace pointer targets an archived workspace

- **WHEN** the app starts with `activeWorkspace` pointing at a workspace archived since
  the pointer was written
- **THEN** the shell opens on the Default workspace and the pointer is rewritten

#### Scenario: Active-workspace pointer targets an absent workspace

- **WHEN** the pointer names a workspace the current list does not carry (deleted, or a
  stale snapshot missing a young workspace)
- **THEN** the shell renders the Default workspace scope without rewriting the pointer,
  which self-heals if the workspace reappears in a later read

#### Scenario: New window seeds from the pointer

- **WHEN** a new main window opens without an explicit window intent
- **THEN** it scopes to the workspace the `activeWorkspace` pointer names, resolved
  against live lifecycle
