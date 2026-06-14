# multi-window

## Purpose

Tear a live panel or a project into its own child window (picture-in-picture) and re-attach it.
Child windows are additional webviews of the same desktop host and backend; detach is a
renderer-runtime concern and never changes the orchestrator seam or the persisted panel tree.

## Requirements

### Requirement: Detach a live panel into a child window

The system SHALL show a detach affordance on a panel header only when that panel hosts a live
surface, and SHALL, on activation, open a child window rendering that same surface and replace
the parent panel with a placeholder.

#### Scenario: Detaching a terminal panel

- **WHEN** the user activates the detach affordance on a panel hosting a live terminal surface
- **THEN** a new child window opens rendering that surface
- **AND** the parent panel is replaced by a greyed placeholder bearing a "Focus" button

#### Scenario: Empty panel exposes no detach

- **WHEN** a panel has no live surface (empty leaf)
- **THEN** no detach affordance is shown on that panel

### Requirement: Detached surface preserves its live session

The system SHALL bind the detached surface in the child window to the same live PTY by its
`(session, placement)` identity, with no new placement minted and no interruption of the
running process.

#### Scenario: Surface continuity across detach

- **WHEN** a panel hosting a running process is detached
- **THEN** the child window renders the same surface with its scrollback
- **AND** the underlying process keeps running uninterrupted

### Requirement: Parent placeholder focuses the child window

The system SHALL render, in place of a detached panel, a placeholder whose "Focus" button
raises the corresponding child window to the front.

#### Scenario: Focusing a detached child from the parent

- **WHEN** the user clicks the "Focus" button on a detached panel's placeholder
- **THEN** the child window hosting that panel is brought to the front

### Requirement: Open a project in a new window

The system SHALL expose, on a sidebar project row, a context-menu action that opens that
project in a child window, and SHALL mark the parent row with a pending-detach indicator that
focuses the child window when clicked.

#### Scenario: Opening a project in a new window

- **WHEN** the user right-clicks a project row and selects "Open in new window"
- **THEN** a child window opens scoped to that project
- **AND** the parent sidebar row shows a pending-detach indicator

#### Scenario: Pending-detach indicator focuses the child

- **WHEN** the user clicks the pending-detach indicator on a project row
- **THEN** the child window for that project is brought to the front

### Requirement: Re-attach returns a detached panel or project to the parent

The system SHALL provide, in a child window, a re-attach action that returns its panel or
project to the parent window, focuses the parent, and closes the child window.

#### Scenario: Re-attaching a detached panel

- **WHEN** the user activates re-attach in a child window hosting a detached panel
- **THEN** the panel is restored in the parent window in place of its placeholder
- **AND** the parent window is focused
- **AND** the child window closes

#### Scenario: Re-attaching a detached project

- **WHEN** the user activates re-attach in a child window hosting a detached project
- **THEN** the project's pending-detach indicator clears in the parent sidebar
- **AND** the parent window is focused
- **AND** the child window closes

### Requirement: Detached windows are independent of the parent

The system SHALL keep detached child windows running when the parent window closes.

#### Scenario: Closing the parent with a detached child open

- **WHEN** the user closes the parent window while a detached child window is open
- **THEN** the child window stays open and its surface keeps running

### Requirement: Detach state is window-runtime only

The system SHALL NOT persist detach state to the session layout; a relaunch SHALL restore each
session in a single window with all panels attached.

#### Scenario: Relaunch after detaching

- **WHEN** the application is relaunched after a panel or project was detached
- **THEN** the session opens in a single window with the panel attached in its layout position
