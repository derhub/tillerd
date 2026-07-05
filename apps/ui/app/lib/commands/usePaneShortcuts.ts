import React from "react";

import { readContext } from "./context";
import { PANE_ACTION_IDS } from "./ids";
import { eventToAccelerator } from "./keybindings";
import { useCommands } from "./registry";
import { useResolvedBindings } from "./useKeybindings";
import { evaluateWhen } from "./when";

// Global shortcuts are suppressed while a terminal holds keyboard focus (useKeybindings'
// isCaptureTarget skips `.xterm`). A focused pane therefore routes its own pane/surface bindings
// (split, close, new, focus, zoom) through the terminal's key handler, which calls the returned
// matcher: it resolves the event against the active keybindings restricted to PANE_ACTION_IDS,
// runs the matching command, and returns true so the caller stops xterm from writing the key to
// the PTY. Mirrors useGlobalShortcuts' match-and-run, minus the capture-target guard.
export function usePaneShortcutDispatch(): (e: KeyboardEvent) => boolean {
  const bindings = useResolvedBindings();
  const commands = useCommands();
  const bindingsRef = React.useRef(bindings);
  bindingsRef.current = bindings;
  const commandsRef = React.useRef(commands);
  commandsRef.current = commands;

  return React.useCallback((e: KeyboardEvent) => {
    const accel = eventToAccelerator(e);
    if (!accel) return false;
    for (const id of PANE_ACTION_IDS) {
      if (bindingsRef.current.get(id) !== accel) continue;
      const command = commandsRef.current.find((c) => c.id === id);
      if (command && evaluateWhen(command.when, readContext())) {
        command.run();
        return true;
      }
      return false;
    }
    return false;
  }, []);
}
