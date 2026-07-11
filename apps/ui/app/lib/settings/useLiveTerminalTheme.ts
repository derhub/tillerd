import type { Terminal } from "@xterm/xterm";

import React from "react";

import { useGlobalSetting } from "./context";
import { TERMINAL_SCHEME_KEY } from "./keys";
import { DEFAULT_TERMINAL_SCHEME, getTerminalTheme, type TerminalTheme } from "./terminal-schemes";

// Single source of truth for "the terminal scheme setting applies live to every mounted
// terminal, no respawn" -- both the desktop pane (DesktopTerminalPane) and the web pane
// (TerminalPane) call this instead of each re-deriving the effect, so a change to one host's
// live-apply behavior cannot silently diverge from the other's (the scenario the
// settings-terminal-scheme e2e spec guards against). `termRef` may still be null when the
// scheme changes before the Terminal instance exists; the effect no-ops until mount populates
// it, and the caller's own initial `theme: terminalTheme` construction option covers that case.
export function useLiveTerminalTheme(termRef: React.RefObject<Terminal | null>): TerminalTheme {
  const { value: scheme } = useGlobalSetting(TERMINAL_SCHEME_KEY, DEFAULT_TERMINAL_SCHEME);
  const terminalTheme = getTerminalTheme(scheme);

  React.useEffect(() => {
    const term = termRef.current;
    if (term) term.options.theme = terminalTheme;
  }, [terminalTheme, termRef]);

  return terminalTheme;
}
