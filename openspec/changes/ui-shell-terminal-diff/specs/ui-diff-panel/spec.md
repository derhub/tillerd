## ADDED Requirements

### Requirement: Persistent right-column placement

The diff panel SHALL occupy the rightmost column of the three-column shell layout. It SHALL remain mounted across session navigation; when no session is active it SHALL display an empty placeholder. Its width is controlled independently by the shell's right resize rail.

#### Scenario: Visible without active session

- **WHEN** no session is selected
- **THEN** the diff panel renders an empty placeholder in the right column

#### Scenario: Persists across session switch

- **WHEN** the user navigates from one session to another
- **THEN** the diff panel column remains mounted; its content updates for the new session

### Requirement: Diff fetch on session completion

The diff panel SHALL fetch the session diff once when it observes the session status transition to IDLE or DONE. It SHALL not fetch on intermediate status values (WORKING, WAITING_INPUT).

#### Scenario: Fetch triggered on IDLE

- **WHEN** the session status message transitions to IDLE
- **THEN** the diff panel issues a request for the session diff and displays the result

#### Scenario: No fetch while working

- **WHEN** the session status is WORKING
- **THEN** the diff panel shows a placeholder and does not issue a diff request

#### Scenario: Re-fetch after subsequent completion

- **WHEN** the session transitions from WORKING back to IDLE a second time (agent ran again)
- **THEN** the diff panel fetches fresh diff data and replaces the previous display

### Requirement: Syntax-highlighted file diff rendering

The diff panel SHALL render each changed file with syntax highlighting and line-level additions/deletions highlighted. File entries SHALL be individually collapsible. A stacked (unified) and split view toggle SHALL be available.

#### Scenario: Files render with syntax highlighting

- **WHEN** the diff panel has loaded patch data containing changed files
- **THEN** each file is rendered with syntax-appropriate token coloring

#### Scenario: File collapse/expand

- **WHEN** the user clicks a file header in the diff panel
- **THEN** that file's diff body collapses or expands

#### Scenario: Stacked/split toggle

- **WHEN** the user activates the view mode toggle
- **THEN** the display switches between stacked (unified) and side-by-side (split) layouts

### Requirement: Empty and loading states

The diff panel SHALL display a loading state while the diff request is in flight and an appropriate empty state when there are no changes or the session's working directory is not a git repository.

#### Scenario: Loading skeleton shown

- **WHEN** the diff request has been issued but not yet resolved
- **THEN** the panel renders a loading indicator

#### Scenario: No changes message

- **WHEN** the diff response contains no changed files
- **THEN** the panel displays a message indicating no changes were detected

#### Scenario: Not a git repo

- **WHEN** the server returns an error indicating the directory is not a git repository
- **THEN** the panel displays an appropriate informational message
