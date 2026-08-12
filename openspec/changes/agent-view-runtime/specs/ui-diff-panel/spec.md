## MODIFIED Requirements

### Requirement: Persistent right-column placement

The diff panel SHALL render inside the workbench right dock rather than claim a panel placement or create a surface. It SHALL remain available across session navigation; when no diff target is active it SHALL display an empty placeholder. Its width is controlled independently by the shell's right resize rail.

#### Scenario: Visible without active session

- **WHEN** no session or diff target is selected
- **THEN** the diff panel renders an empty placeholder in the right dock

#### Scenario: Persists across session switch

- **WHEN** the user navigates from one session to another
- **THEN** the right dock remains mounted; the diff panel updates for the new session without altering surface ownership or placement

### Requirement: Diff fetch on session completion

The diff panel SHALL request a structured diff for the active project's selected bounded target when that target becomes available or changes. It SHALL not request a diff for an intermediate session state unless that state supplies a selected target.

#### Scenario: Fetch triggered on IDLE

- **WHEN** the active session transitions to IDLE and exposes a selected diff target
- **THEN** the diff panel requests the structured diff for that target and displays the result

#### Scenario: No fetch while working

- **WHEN** the session is WORKING and no selected diff target is available
- **THEN** the diff panel shows a placeholder and does not issue a diff request

#### Scenario: Re-fetch after subsequent completion

- **WHEN** the active session exposes a different selected diff target after returning to IDLE
- **THEN** the diff panel requests fresh structured diff data and replaces the previous display

### Requirement: Syntax-highlighted file diff rendering

The diff panel SHALL render each changed file from the structured diff model with syntax highlighting and line-level additions and deletions highlighted. File entries SHALL be individually collapsible. A stacked (unified) and split view toggle SHALL be available.

#### Scenario: Files render with syntax highlighting

- **WHEN** the diff panel has loaded a structured diff model containing changed files
- **THEN** each file is rendered with syntax-appropriate token coloring

#### Scenario: File collapse/expand

- **WHEN** the user clicks a file header in the diff panel
- **THEN** that file's diff body collapses or expands

#### Scenario: Stacked/split toggle

- **WHEN** the user activates the view mode toggle
- **THEN** the display switches between stacked (unified) and side-by-side (split) layouts
