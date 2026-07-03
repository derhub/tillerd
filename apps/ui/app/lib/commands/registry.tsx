import type { ReactNode } from "react";

import { useSelector } from "@tanstack/react-store";
import React from "react";

import { contextStore } from "./context";
import { COMMAND_DEFS } from "./defs";
import {
  isOnSurface,
  type Command,
  type CommandDef,
  type CommandHandler,
  type Surface,
} from "./types";
import { evaluateWhen, type ContextSnapshot } from "./when";

export type { Command, CommandDef, CommandHandler, Surface } from "./types";

const NOOP: CommandHandler = () => {};

interface RegistryDispatch {
  register: (token: string, handlers: Record<string, CommandHandler>) => void;
  unregister: (token: string) => void;
}

// Dispatch is split from the handler map so a contributor does not re-render when
// the merged map changes -- registering re-renders the registrant, whose handler
// record may not be referentially stable, causing the register effect to loop.
const HandlerDispatchContext = React.createContext<RegistryDispatch | null>(null);
const HandlersContext = React.createContext<ReadonlyMap<string, CommandHandler>>(new Map());

export function CommandRegistryProvider({ children }: { children: ReactNode }) {
  const [sources, setSources] = React.useState<Record<string, Record<string, CommandHandler>>>({});

  const register = React.useCallback((token: string, handlers: Record<string, CommandHandler>) => {
    setSources((prev) => ({ ...prev, [token]: handlers }));
  }, []);

  const unregister = React.useCallback((token: string) => {
    setSources((prev) => {
      if (!(token in prev)) return prev;
      const next = { ...prev };
      delete next[token];
      return next;
    });
  }, []);

  const handlers = React.useMemo(() => {
    const byId = new Map<string, CommandHandler>();
    for (const record of Object.values(sources)) {
      for (const [id, handler] of Object.entries(record)) byId.set(id, handler);
    }
    return byId;
  }, [sources]);

  const dispatch = React.useMemo<RegistryDispatch>(
    () => ({ register, unregister }),
    [register, unregister],
  );

  return (
    <HandlerDispatchContext value={dispatch}>
      <HandlersContext value={handlers}>{children}</HandlersContext>
    </HandlerDispatchContext>
  );
}

// Pass a memoized record -- an unstable identity re-runs the register effect.
export function useRegisterHandlers(handlers: Record<string, CommandHandler>): void {
  const dispatch = React.use(HandlerDispatchContext);
  const token = React.useId();
  const register = dispatch?.register;
  const unregister = dispatch?.unregister;

  React.useEffect(() => {
    if (!register || !unregister) return;
    register(token, handlers);
    return () => unregister(token);
  }, [register, unregister, token, handlers]);
}

// Register a single command's handler by id. `handler` should be stable
// (wrap in useCallback) so the registration does not churn each render.
export function useCommand(id: string, handler: CommandHandler): void {
  const record = React.useMemo(() => ({ [id]: handler }), [id, handler]);
  useRegisterHandlers(record);
}

export function RegisterHandlers({ handlers }: { handlers: Record<string, CommandHandler> }): null {
  useRegisterHandlers(handlers);
  return null;
}

// Pure composition: each definition gets its resolved handler (NOOP if none) and,
// for toggles, its checked state evaluated against the context snapshot.
export function composeCommands(
  defs: readonly CommandDef[],
  handlers: ReadonlyMap<string, CommandHandler>,
  ctx: ContextSnapshot,
): Command[] {
  return defs.map((def) => ({
    ...def,
    run: handlers.get(def.id) ?? NOOP,
    checked: def.toggle ? def.toggle(ctx) : undefined,
  }));
}

// Every declared command composed with its resolved handler and checked state.
// Not filtered by `when` -- callers that render a surface use useSurfaceCommands.
export function useCommands(): Command[] {
  const handlers = React.use(HandlersContext);
  const ctx = useSelector(contextStore, (s) => s);
  return React.useMemo(() => composeCommands(COMMAND_DEFS, handlers, ctx), [handlers, ctx]);
}

// Commands tagged for a surface whose `when` currently passes.
export function useSurfaceCommands(surface: Surface): Command[] {
  const commands = useCommands();
  const ctx = useSelector(contextStore, (s) => s);
  return React.useMemo(
    () => commands.filter((c) => isOnSurface(c, surface) && evaluateWhen(c.when, ctx)),
    [commands, ctx, surface],
  );
}
