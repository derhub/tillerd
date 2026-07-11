export const ACTION = {
  projectNew: "project.new",
  sessionNew: "session.new",
  surfaceSpawn: "surface.spawn",
  // Spawn a library command into the workbench panel tree. Registered by PanelContent
  // (the tree owner) and dispatched by out-of-tree surfaces like the commands sidebar;
  // handler-only (no COMMAND_DEFS entry) so it is not a palette/keyboard command.
  surfaceRunCommand: "surface.run-command",
  surfaceClose: "surface.close",
  panelSplitH: "panel.split-h",
  panelSplitV: "panel.split-v",
  surfaceDetach: "surface.detach",
  surfaceNew: "surface.new",
  paneFocusLeft: "pane.focus-left",
  paneFocusRight: "pane.focus-right",
  paneFocusUp: "pane.focus-up",
  paneFocusDown: "pane.focus-down",
  paneZoomToggle: "pane.zoom-toggle",
  projectOpenNewWindow: "project.open-new-window",
  viewLogs: "view.logs",
  appSettings: "app.settings",
  viewZoomIn: "view.zoom-in",
  viewZoomOut: "view.zoom-out",
  viewZoomReset: "view.zoom-reset",
  // Sidebar view-switch actions (activity bar + palette). Selecting the active
  // view toggles the sidebar (see useWorkbenchCommands).
  viewSessions: "view.sessions",
  viewSearch: "view.search",
  viewCommands: "view.commands",
  viewTemplates: "view.templates",
  panelToggleLeft: "panel.toggle-left",
  panelToggleBottom: "panel.toggle-bottom",
  commandToggle: "command.toggle",
  // Row-scoped context-menu actions. Distinct from `projectOpenNewWindow`
  // (palette/keyboard, acts on the active project) because a def's `when` gates
  // every surface it's tagged for -- a shared id/def would either hide the
  // shortcut outside an open menu or require a second def with the same id,
  // which corrupts the id-keyed maps `useGlobalShortcuts`/`COMMAND_DEFS_BY_ID`
  // build (last entry silently wins). See EntityContextMenu.
  projectOpenNewWindowRow: "project.open-new-window-row",
  projectNewSessionFromTemplate: "project.new-session-from-template",
  projectRename: "project.rename",
  projectDuplicate: "project.duplicate",
  projectPin: "project.pin",
  projectUnpin: "project.unpin",
  projectMove: "project.move",
  projectStopSurfaces: "project.stop-surfaces",
  projectArchive: "project.archive",
  projectDelete: "project.delete",
  sessionRename: "session.rename",
  sessionDuplicate: "session.duplicate",
  sessionPin: "session.pin",
  sessionUnpin: "session.unpin",
  sessionMove: "session.move",
  sessionStopSurfaces: "session.stop-surfaces",
  sessionArchive: "session.archive",
  sessionDelete: "session.delete",
  workspaceRename: "workspace.rename",
  workspacePin: "workspace.pin",
  workspaceUnpin: "workspace.unpin",
  workspaceStopSurfaces: "workspace.stop-surfaces",
  workspaceArchive: "workspace.archive",
  workspaceDelete: "workspace.delete",
  // -- command library row (see CommandsView) --
  commandRun: "command.run",
  commandEdit: "command.edit",
  commandRename: "command.rename",
  commandDuplicate: "command.duplicate",
  commandPin: "command.pin",
  commandUnpin: "command.unpin",
  commandDelete: "command.delete",
  // -- portable template library row (see TemplatesView) --
  templatePin: "template.pin",
  templateUnpin: "template.unpin",
  templateExport: "template.export",
  templateDelete: "template.delete",
  // -- project launch-template row (see TemplatesView) --
  launchTemplateEdit: "launch-template.edit",
  launchTemplateDiscard: "launch-template.discard",
  // -- terminal pane (find overlay + context menu; see TerminalPane/DesktopTerminalPane) --
  // Handlers register centrally (PanelContent) and act on the focused terminal via
  // activeTerminalStore, so a single id serves every mounted pane without collision.
  terminalFind: "terminal.find",
  terminalCopy: "terminal.copy",
  terminalPaste: "terminal.paste",
  terminalSelectAll: "terminal.select-all",
  terminalClear: "terminal.clear",
  terminalSearchSelection: "terminal.search-selection",
} as const;

export type ActionId = (typeof ACTION)[keyof typeof ACTION];

// Action ids that are dispatched imperatively (useDispatchCommand) but deliberately
// carry no COMMAND_DEFS entry, so they never appear in the palette or bind a shortcut.
// The def-parity test asserts these -- and only these -- are absent from the def table.
export const HANDLER_ONLY_ACTION_IDS: ReadonlySet<string> = new Set([ACTION.surfaceRunCommand]);

// Pane/surface actions that operate on the focused leaf and must fire while a terminal holds
// keyboard focus. Global shortcuts are suppressed over a focused `.xterm` (useKeybindings
// isCaptureTarget), so these are matched inside the terminal's own key handler and dispatched
// through the registry (see usePaneShortcutDispatch).
export const PANE_ACTION_IDS: readonly string[] = [
  ACTION.panelSplitH,
  ACTION.panelSplitV,
  ACTION.surfaceClose,
  ACTION.surfaceNew,
  ACTION.surfaceDetach,
  ACTION.paneFocusLeft,
  ACTION.paneFocusRight,
  ACTION.paneFocusUp,
  ACTION.paneFocusDown,
  ACTION.paneZoomToggle,
];

export const SESSION_SEARCH_ACTION_ID = "session.search";
export const SESSION_SEARCH_TITLE = "Search sessions";
