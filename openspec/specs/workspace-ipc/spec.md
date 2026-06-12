# workspace-ipc Specification

## Purpose

Workspace IPC defines the host control surface that bridges the renderer to workspace store operations. Every store operation reachable from the renderer must have a corresponding host command; typed errors must cross the boundary as serializable results rather than panics.

## Requirements

### Requirement: The host exposes every workspace store operation

The host control surface SHALL expose to the renderer the full set of workspace operations:
project create, rename, list, and archive; session create, rename, list, archive, and layout
get/set; and command-library list, create, get, and delete. A store operation reachable from the
renderer's client SHALL have a corresponding host command; a client call with no host handler is an
incomplete control surface.

#### Scenario: Project lifecycle is fully callable

- **WHEN** the renderer creates, renames, lists, and archives a project
- **THEN** each call reaches the corresponding store operation and returns its result

#### Scenario: Session lifecycle and layout are fully callable

- **WHEN** the renderer creates, renames, lists, and archives a session and sets and gets its layout
- **THEN** each call reaches the corresponding store operation

#### Scenario: Command-library is fully callable

- **WHEN** the renderer lists, creates, gets, and deletes a command-library entry
- **THEN** each call reaches the corresponding store operation

### Requirement: Workspace errors cross the control surface as typed results

A failed workspace operation SHALL return a typed, serializable error to the renderer rather than a
panic or an opaque failure.

#### Scenario: A not-found error is typed

- **WHEN** the renderer requests an operation on an absent identifier
- **THEN** a typed not-found error is returned to the renderer
