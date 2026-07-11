import React from "react";

import type { CommandHandler } from "~/lib/commands/registry";

import { VIEW_DEFS } from "~/components/workbench/views";
import { setContextKey } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { useRegisterHandlers } from "~/lib/commands/registry";
import { useCommandCenterOpen } from "~/lib/store";
import { useBottomPanelVisible, useSidebarVisible, useWorkbenchView } from "~/lib/workbench";

// Wires the workbench chrome commands to live state: seeds the context keys that
// drive `checked`/`toggle` state (active view, sidebar/bottom-panel visibility,
// palette open) and registers the handlers for view switching and region toggles.
// Mount once in the shell; the commands stay active regardless of which surface
// renders them (activity bar, title bar, status bar, palette).
export function useWorkbenchCommands(): void {
  const [view, setView] = useWorkbenchView();
  const [sidebarVisible, setSidebarVisible] = useSidebarVisible();
  const [bottomVisible, setBottomVisible] = useBottomPanelVisible();
  const [commandCenterOpen, setCommandCenterOpen] = useCommandCenterOpen();

  // Seed synchronously before paint so a command's checked state is correct on the
  // first render (a post-paint effect would flash a default state for a frame).
  React.useLayoutEffect(() => {
    setContextKey("activeView", view);
    setContextKey("sidebarVisible", sidebarVisible);
    setContextKey("bottomPanelVisible", bottomVisible);
    setContextKey("commandPaletteOpen", commandCenterOpen);
  }, [view, sidebarVisible, bottomVisible, commandCenterOpen]);

  const handlers = React.useMemo<Record<string, CommandHandler>>(() => {
    const record: Record<string, CommandHandler> = {
      [ACTION.panelToggleLeft]: () => setSidebarVisible(!sidebarVisible),
      [ACTION.panelToggleBottom]: () => setBottomVisible(!bottomVisible),
      [ACTION.commandToggle]: () => setCommandCenterOpen(!commandCenterOpen),
    };
    for (const def of VIEW_DEFS) {
      // Selecting the active view toggles the sidebar; selecting another view
      // switches to it and reveals the sidebar if it was hidden.
      record[def.commandId] = () => {
        if (def.id === view) {
          setSidebarVisible(!sidebarVisible);
        } else {
          setView(def.id);
          if (!sidebarVisible) setSidebarVisible(true);
        }
      };
    }
    return record;
  }, [
    view,
    sidebarVisible,
    bottomVisible,
    commandCenterOpen,
    setView,
    setSidebarVisible,
    setBottomVisible,
    setCommandCenterOpen,
  ]);

  useRegisterHandlers(handlers);
}
