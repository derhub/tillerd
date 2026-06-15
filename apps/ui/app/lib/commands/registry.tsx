import { createContext, useCallback, useContext, useEffect, useId, useMemo, useState } from "react";
import type { ReactNode } from "react";

/** A single invocable action. `run` is the same handler the action's UI control calls. */
export interface Command {
  /** Stable id; bindable static ids live in `./ids`, dynamic entries are namespaced (e.g. `session.switch:<id>`). */
  id: string;
  title: string;
  /** Extra terms the fuzzy search should match besides the title. */
  keywords?: string[];
  /** Optional group heading in the palette. */
  group?: string;
  run: () => void;
}

interface RegistryValue {
  register: (token: string, commands: Command[]) => void;
  unregister: (token: string) => void;
  commands: Command[];
}

const CommandRegistryContext = createContext<RegistryValue | null>(null);

/**
 * Collects commands contributed by mounted components. Each contributor owns a token (one per
 * `useRegisterCommands` call); the merged list dedupes by id (last contributor wins) so the palette
 * has a single read surface and new actions register additively.
 */
export function CommandRegistryProvider({ children }: { children: ReactNode }) {
  const [sources, setSources] = useState<Record<string, Command[]>>({});

  const register = useCallback((token: string, commands: Command[]) => {
    setSources((prev) => ({ ...prev, [token]: commands }));
  }, []);

  const unregister = useCallback((token: string) => {
    setSources((prev) => {
      if (!(token in prev)) return prev;
      const next = { ...prev };
      delete next[token];
      return next;
    });
  }, []);

  const commands = useMemo(() => {
    const byId = new Map<string, Command>();
    for (const list of Object.values(sources)) {
      for (const command of list) byId.set(command.id, command);
    }
    return [...byId.values()];
  }, [sources]);

  const value = useMemo<RegistryValue>(
    () => ({ register, unregister, commands }),
    [register, unregister, commands],
  );

  return <CommandRegistryContext value={value}>{children}</CommandRegistryContext>;
}

/**
 * Contribute commands while mounted. Pass a memoized array — a new array each render re-registers
 * (cheap, same token) but an unstable identity that also derives from `commands` could loop.
 */
export function useRegisterCommands(commands: Command[]): void {
  const ctx = useContext(CommandRegistryContext);
  const token = useId();
  const register = ctx?.register;
  const unregister = ctx?.unregister;

  useEffect(() => {
    if (!register || !unregister) return;
    register(token, commands);
    return () => unregister(token);
  }, [register, unregister, token, commands]);
}

/** The merged, deduped command list for the palette. Empty without a provider. */
export function useCommands(): Command[] {
  return useContext(CommandRegistryContext)?.commands ?? [];
}

/** Declarative contributor for a memoized command array built by a parent. Renders nothing. */
export function RegisterCommands({ commands }: { commands: Command[] }): null {
  useRegisterCommands(commands);
  return null;
}
