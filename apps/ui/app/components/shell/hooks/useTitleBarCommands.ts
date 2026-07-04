import React from "react";

import { setContextKey } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { useCommand } from "~/lib/commands/registry";
import { useCommandCenterOpen, usePanelVisible } from "~/lib/store";

// Wires the title-bar toggle commands to live state: seeds each command's
// checked-state context key from the durable panel-visibility settings (and the
// command-center open flag), and registers the handler that flips it. Mount once
// in the shell -- the commands stay active regardless of which surface renders
// them.
export function useTitleBarCommands(): void {
  const [leftVisible, setLeftVisible] = usePanelVisible("left");
  const [rightVisible, setRightVisible] = usePanelVisible("right");
  const [bottomVisible, setBottomVisible] = usePanelVisible("bottom");
  const [commandCenterOpen, setCommandCenterOpen] = useCommandCenterOpen();

  // Seed synchronously before paint (layout effect) so a toggle's checked state
  // is correct on first render -- a plain effect flushes post-paint and would flash
  // a default-visible region's button as unchecked for a frame.
  React.useLayoutEffect(() => {
    setContextKey("leftPanelVisible", leftVisible);
    setContextKey("rightPanelVisible", rightVisible);
    setContextKey("bottomPanelVisible", bottomVisible);
    setContextKey("commandPaletteOpen", commandCenterOpen);
  }, [leftVisible, rightVisible, bottomVisible, commandCenterOpen]);

  useCommand(
    ACTION.panelToggleLeft,
    React.useCallback(() => setLeftVisible(!leftVisible), [setLeftVisible, leftVisible]),
  );
  useCommand(
    ACTION.panelToggleRight,
    React.useCallback(() => setRightVisible(!rightVisible), [setRightVisible, rightVisible]),
  );
  useCommand(
    ACTION.panelToggleBottom,
    React.useCallback(() => setBottomVisible(!bottomVisible), [setBottomVisible, bottomVisible]),
  );
  useCommand(
    ACTION.commandToggle,
    React.useCallback(
      () => setCommandCenterOpen(!commandCenterOpen),
      [setCommandCenterOpen, commandCenterOpen],
    ),
  );
}
