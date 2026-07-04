// Whether a paste should be held for confirmation: only when the guard setting is on and the
// clipboard carries a line break, since a multi-line paste can submit commands the moment it
// reaches the PTY (ui-terminal-pane "Multi-line paste confirmation").
export function shouldConfirmPaste(text: string, confirmEnabled: boolean): boolean {
  if (!confirmEnabled) return false;
  return text.includes("\n") || text.includes("\r");
}
