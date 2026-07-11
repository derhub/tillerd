import React from "react";

import { readContext } from "~/lib/commands/context";
import { useCommands } from "~/lib/commands/registry";
import { evaluateWhen } from "~/lib/commands/when";
import { subscribe } from "~/lib/subscribe";
import { isDesktopHost } from "~/lib/transport";
import { loadTauriCore } from "~/lib/transport/core";

function listenMenuCommand(handler: (id: string) => void): Promise<() => void> {
  return loadTauriCore().then((core) => core.listen<string>("menu:command", handler));
}

// Native menu items (File > New Project, etc.) emit "menu:command" with a palette command id
// as payload (see menu.rs) -- this dispatches through the same registry the palette uses, so a
// menu item and its palette entry can never disagree on behavior. The event is OS-level and
// reaches the app regardless of webview focus, so it fires while a terminal surface is focused.
export function useMenuCommands(): void {
  const commands = useCommands();
  const commandsRef = React.useRef(commands);
  commandsRef.current = commands;

  React.useEffect(() => {
    if (!isDesktopHost()) return;
    return subscribe(
      listenMenuCommand((id) => {
        const command = commandsRef.current.find((c) => c.id === id);
        if (command && evaluateWhen(command.when, readContext())) command.run();
      }),
    );
  }, []);
}
