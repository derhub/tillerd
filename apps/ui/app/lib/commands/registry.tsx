import type { ReactNode } from "react";

import React from "react";

export interface Command {
  id: string;
  title: string;
  keywords?: string[];
  group?: string;
  run: () => void;
}

interface RegistryDispatch {
  register: (token: string, commands: Command[]) => void;
  unregister: (token: string) => void;
}

// Dispatch is split from the command list so a contributor does not re-render when the merged list
// changes -- registering re-renders the registrant, whose command array may not be referentially
// stable, causing the register effect to loop.
const CommandDispatchContext = React.createContext<RegistryDispatch | null>(null);
const CommandsContext = React.createContext<Command[]>([]);

export function CommandRegistryProvider({ children }: { children: ReactNode }) {
  const [sources, setSources] = React.useState<Record<string, Command[]>>({});

  const register = React.useCallback((token: string, commands: Command[]) => {
    setSources((prev) => ({ ...prev, [token]: commands }));
  }, []);

  const unregister = React.useCallback((token: string) => {
    setSources((prev) => {
      if (!(token in prev)) return prev;
      const next = { ...prev };
      delete next[token];
      return next;
    });
  }, []);

  const commands = React.useMemo(() => {
    const byId = new Map<string, Command>();
    for (const list of Object.values(sources)) {
      for (const command of list) byId.set(command.id, command);
    }
    return [...byId.values()];
  }, [sources]);

  // Stable for the provider's lifetime so dispatch consumers never re-render on a command-list change.
  const dispatch = React.useMemo<RegistryDispatch>(
    () => ({ register, unregister }),
    [register, unregister],
  );

  return (
    <CommandDispatchContext value={dispatch}>
      <CommandsContext value={commands}>{children}</CommandsContext>
    </CommandDispatchContext>
  );
}

// Pass a memoized array -- an unstable identity that also derives from `commands` could loop.
export function useRegisterCommands(commands: Command[]): void {
  const dispatch = React.use(CommandDispatchContext);
  const token = React.useId();
  const register = dispatch?.register;
  const unregister = dispatch?.unregister;

  React.useEffect(() => {
    if (!register || !unregister) return;
    register(token, commands);
    return () => unregister(token);
  }, [register, unregister, token, commands]);
}

export function useCommands(): Command[] {
  return React.use(CommandsContext);
}

export function RegisterCommands({ commands }: { commands: Command[] }): null {
  useRegisterCommands(commands);
  return null;
}
