// Terminal links open on modifier+click (Cmd on macOS, Ctrl elsewhere) so a plain click keeps
// selecting text -- matches the ui-terminal-pane "activates it with the platform modifier" scenario.
export function linkModifierHeld(
  e: { metaKey: boolean; ctrlKey: boolean },
  isMac: boolean,
): boolean {
  return isMac ? e.metaKey : e.ctrlKey;
}

export type TerminalKeyAction = "find" | "copy" | "paste" | null;

export interface TerminalKeyEvent {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

// Which pane-owned action a key event maps to, if any. Everything else (notably bare Ctrl+C)
// returns null and stays with the PTY. On macOS the Cmd modifier owns find/copy/paste; elsewhere
// the terminal convention is Ctrl for find and Ctrl+Shift for copy/paste so Ctrl+C remains SIGINT.
export function classifyTerminalKey(e: TerminalKeyEvent, isMac: boolean): TerminalKeyAction {
  const k = e.key.toLowerCase();
  if (isMac) {
    if (!e.metaKey || e.ctrlKey) return null;
    if (k === "f") return "find";
    if (k === "c") return "copy";
    if (k === "v") return "paste";
    return null;
  }
  if (!e.ctrlKey || e.metaKey) return null;
  if (k === "f" && !e.shiftKey) return "find";
  if (k === "c" && e.shiftKey) return "copy";
  if (k === "v" && e.shiftKey) return "paste";
  return null;
}
