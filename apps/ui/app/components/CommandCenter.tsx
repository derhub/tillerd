import { useCallback, useEffect, useState } from "react";

import {
  Command as CommandBox,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "~/components/ui/command";
import { displayAccelerator } from "~/lib/commands/keybindings";
import { useCommands, type Command } from "~/lib/commands/registry";
import {
  useGlobalShortcuts,
  useLeaderBinding,
  useResolvedBindings,
} from "~/lib/commands/useKeybindings";
import { COMMAND_CENTER_OPEN_EVENT, loadLeaderKeyPort } from "~/lib/transport/leader-source";
import { useWindowEvent } from "~/lib/useWindowEvent";

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);

/**
 * Leader-activated command palette. Opens on the desktop native leader accelerator (via the leader
 * port) or an in-renderer `command-center:open` event (web host, tests, programmatic). Lists the
 * registry's commands with fuzzy search and their resolved key hints; selecting one runs the same
 * handler its UI control calls. Also installs the in-renderer per-action shortcuts.
 */
export function CommandCenter() {
  const [open, setOpen] = useState(false);
  const commands = useCommands();
  const bindings = useResolvedBindings();
  const leader = useLeaderBinding();

  useGlobalShortcuts(bindings);

  // In-renderer open signal — uniform across hosts and reachable from tests / e2e.
  useWindowEvent(COMMAND_CENTER_OPEN_EVENT, () => setOpen(true));

  // Desktop native leader: keep the accelerator in sync and open on activation.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void (async () => {
      const port = await loadLeaderKeyPort();
      if (!port || cancelled) return;
      const [, listen] = await Promise.all([
        port.setBinding(leader),
        port.onActivate(() => setOpen(true)),
      ]);
      unlisten = listen;
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [leader]);

  const invoke = useCallback((command: Command) => {
    setOpen(false);
    command.run();
  }, []);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-black/40 pt-[12vh] animate-in fade-in-0"
      onMouseDown={() => setOpen(false)}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          setOpen(false);
        }
      }}
      data-testid="command-center"
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-md border border-border/60 bg-popover shadow-lg"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <CommandBox loop>
          <CommandInput
            autoFocus
            placeholder="Search actions…"
            data-testid="command-center-input"
          />
          <CommandList>
            <CommandEmpty>No actions</CommandEmpty>
            {commands.map((command) => {
              const accel = bindings.get(command.id);
              return (
                <CommandItem
                  key={command.id}
                  value={command.title}
                  keywords={command.keywords}
                  onSelect={() => invoke(command)}
                >
                  <span>{command.title}</span>
                  {accel && <CommandShortcut>{displayAccelerator(accel, isMac)}</CommandShortcut>}
                </CommandItem>
              );
            })}
          </CommandList>
        </CommandBox>
      </div>
    </div>
  );
}
