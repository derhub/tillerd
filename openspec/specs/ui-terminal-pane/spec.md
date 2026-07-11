# ui-terminal-pane

## Purpose

Terminal pane attached to a session's terminal surface over the host-mapped transport -- the duplex `channel` verb on desktop (Tauri IPC), a WebSocket on the server host -- rendering raw output, propagating resize, and surfacing connection status with a manual reconnect control.
## Requirements
### Requirement: Session-scoped terminal connection

The terminal pane SHALL accept the identifier of a session's terminal surface as input and attach to
that surface's output stream through the orchestrator client. When the surface identifier changes,
the previous attachment SHALL be torn down and a new one established.

#### Scenario: Connection opens for surface

- **WHEN** the terminal pane mounts with a given surface identifier
- **THEN** it attaches to that surface through the orchestrator client and begins streaming output

#### Scenario: Reattach on surface change

- **WHEN** the active surface identifier changes (e.g. the user navigates to a different session)
- **THEN** the prior attachment is closed, the terminal buffer is cleared, and a new attachment opens for the new surface identifier

#### Scenario: Teardown on unmount

- **WHEN** the terminal pane unmounts
- **THEN** the attachment is closed cleanly and the surface's pseudo-terminal keeps running for later resume

### Requirement: Terminal output rendering

The terminal pane SHALL render surface output as it arrives, preserving ANSI escape sequences and
raw bytes. The pane SHALL auto-fit to its container's dimensions and send a resize for the surface to
the orchestrator through the client when the container size changes.

#### Scenario: Output streams to display

- **WHEN** the orchestrator delivers output bytes for the surface
- **THEN** the terminal renders them immediately including colors, cursor movement, and other control sequences

#### Scenario: Resize propagates

- **WHEN** the terminal container is resized (window resize or panel drag)
- **THEN** the pane recalculates dimensions and sends a resize for the surface to the orchestrator through the client

### Requirement: Connection status indicator

The terminal pane SHALL display a visual status indicator reflecting the current attachment state to
the surface stream: connecting, connected, disconnected, or exited. The pane SHALL provide a manual
reconnect control that reattaches to the surface.

When the surface's process has exited cleanly, the pane SHALL keep its final scrollback visible and
present an inline exit affordance that reports the exit qualifier and offers a Restart control and a
New surface control. Restart SHALL rebind the pane by re-resolving its `(session, placement)`, which
resumes the surface in place with a fresh pseudo-terminal (same surface id and placement) without
recreating the terminal view. New surface SHALL request the leaf be reset to an empty picker. The pane SHALL NOT auto-clear or auto-remove on a clean exit; it changes only when the
user acts on the exit affordance or closes the pane.

#### Scenario: Status updates on connection events

- **WHEN** the attachment to the surface stream transitions between states
- **THEN** the status indicator updates accordingly (e.g. yellow while connecting, green when connected, red when disconnected)

#### Scenario: Manual reconnect

- **WHEN** the user activates the reconnect control
- **THEN** the existing attachment is closed, the terminal buffer is cleared, and a new attachment to the surface opens

#### Scenario: Clean exit shows the exit affordance

- **WHEN** the surface's process exits cleanly
- **THEN** the pane keeps its final output visible and shows an inline exit affordance reporting the exit code with Restart and New surface controls

#### Scenario: Restart resumes the placement in place

- **WHEN** the user activates Restart on an exited pane
- **THEN** the pane re-resolves its `(session, placement)`, a fresh pseudo-terminal is spawned for the same surface id and placement, and the pane rebinds its data channel without recreating the terminal view

### Requirement: Find in terminal

Each terminal pane SHALL offer a search overlay over its scrollback (open via keybinding,
palette, and the pane context menu) with match highlighting, next/previous navigation,
and case-sensitivity toggle; Escape dismisses it returning focus to the terminal.

#### Scenario: Searching scrollback

- **WHEN** the user opens find in a terminal with matching output and steps next
- **THEN** matches highlight and the viewport scrolls to each match in turn

### Requirement: Clickable links

URLs in terminal output (detected or OSC 8 hyperlinks) SHALL be activatable, opening in
the system browser, with a hover affordance.

#### Scenario: Opening a printed URL

- **WHEN** output contains a URL and the user activates it with the platform modifier
- **THEN** the system browser opens that URL

### Requirement: Clipboard hygiene

The terminal SHALL support a copy-on-select setting, preserve bracketed paste, and — when
the corresponding setting is enabled — prompt before pasting multi-line clipboard content.

#### Scenario: Multi-line paste confirmation

- **WHEN** confirm-before-paste is enabled and the user pastes content containing a
  newline
- **THEN** a confirmation shows the content before it reaches the PTY

### Requirement: Terminal typography and buffer settings apply live

Font size, font family, line height, cursor style, cursor blink, and scrollback size
SHALL be user-settable and apply to mounted terminals without restart.

#### Scenario: Font size change

- **WHEN** the user changes the terminal font size in settings
- **THEN** every mounted terminal re-renders at the new size immediately

### Requirement: Terminal context menu

Right-clicking a terminal pane SHALL offer copy, paste, select all, clear, and search
selection, driven by the command registry.

#### Scenario: Copy from the context menu

- **WHEN** the user right-clicks a selection and picks copy
- **THEN** the selection is on the system clipboard

### Requirement: Bell surfaces as a notification

A terminal bell SHALL surface through the notification center (and the native banner
path when the window is unfocused), attributed to its session.

#### Scenario: Bell while unfocused

- **WHEN** a bell rings in a background session while the app is unfocused
- **THEN** a notification records it with the session context

### Requirement: Path drop

Dropping a file onto a terminal pane SHALL insert its shell-quoted path at the cursor.

#### Scenario: Dropping a file

- **WHEN** the user drags a file from the system file manager onto a terminal
- **THEN** the file's quoted absolute path is written to the PTY input

### Requirement: Default terminal typography and color scheme

The terminal SHALL render with a bundled default monospace font and the user-selected
color scheme from settings, applied live to existing and new terminals. The scheme maps
onto the terminal token slots; the terminal canvas stays dark in both host themes.

#### Scenario: Bundled font renders offline

- **WHEN** a terminal surface renders with no network access
- **THEN** the bundled monospace font is used (no system-font fallback flash)

#### Scenario: Scheme change applies live

- **WHEN** the user selects a different terminal color scheme in settings
- **THEN** running terminals re-render with the new scheme without restart

### Requirement: Copy and paste

Terminal selection copy and clipboard paste SHALL work with the platform's standard
shortcuts on macOS and Linux, with no Tauri webview conflicts.

#### Scenario: Copy from selection

- **WHEN** the user selects terminal output and presses the platform copy shortcut
- **THEN** the selected text is on the system clipboard

#### Scenario: Paste into the terminal

- **WHEN** the user presses the platform paste shortcut with text on the clipboard
- **THEN** the text is written to the PTY input

### Requirement: Surface failure overlay

A surface-level failure (spawn error, abnormal exit) SHALL render an overlay inside the
pane using the terminal token set, showing the failure reason with resume/dismiss
actions, visually distinct from the service-health indicator.

#### Scenario: Abnormal exit shows the overlay

- **WHEN** a terminal's process exits abnormally
- **THEN** an overlay styled with terminal tokens presents the exit reason and a resume
  action

