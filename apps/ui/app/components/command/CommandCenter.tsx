import { runCommand, subscribe as eventSub } from "@tillerd/client-bindings";
import { Check } from "lucide-react";
import React from "react";

import {
  Command as CommandBox,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "~/components/ui/command";
import { displayAccelerator } from "~/lib/commands/keybindings";
import { useSurfaceCommands, type Command } from "~/lib/commands/registry";
import {
  useGlobalShortcuts,
  useLeaderBinding,
  useResolvedBindings,
} from "~/lib/commands/useKeybindings";
import { isMac } from "~/lib/platform";
import { useCommandCenterOpen } from "~/lib/store";
import { subscribe } from "~/lib/subscribe";
import { useWindowEvent } from "~/lib/useWindowEvent";

async function mountLeaderKey(leader: string, onActivate: () => void): Promise<() => void> {
  const [, unlisten] = await Promise.all([
    runCommand("commandCenterSetLeader", { accelerator: leader }),
    eventSub("commandCenterOpen").listen(() => onActivate()),
  ]);
  return unlisten;
}

export function CommandCenter() {
  const [open, setOpen] = useCommandCenterOpen();
  const commands = useSurfaceCommands("palette");
  const bindings = useResolvedBindings();
  const leader = useLeaderBinding();

  useGlobalShortcuts(bindings);

  // In-renderer open signal - uniform across hosts and reachable from tests / e2e.
  useWindowEvent("command-center:open", () => setOpen(true));

  React.useEffect(() => subscribe(mountLeaderKey(leader, () => setOpen(true))), [leader]);

  const invoke = React.useCallback((command: Command) => {
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
                  {command.toggle && (
                    <Check
                      size={14}
                      className={command.checked ? "opacity-100" : "opacity-0"}
                      aria-hidden
                    />
                  )}
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
