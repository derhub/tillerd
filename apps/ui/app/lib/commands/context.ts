// Reactive context-key store. Feature code pushes named flags in via
// setContextKey (VSCode's setContext model); the command layer reads them to
// evaluate `when` availability and toggle checked-state. Keeping this the single
// reactive surface means the palette and toolbars re-render uniformly on change,
// instead of subscribing to every store a command might depend on.

import { Store, useSelector } from "@tanstack/react-store";

import { evaluateWhen, type ContextSnapshot, type ContextValue, type WhenExpr } from "./when";

export const contextStore = new Store<Record<string, ContextValue | undefined>>({});

export function setContextKey(key: string, value: ContextValue | undefined): void {
  contextStore.setState((s) => {
    if (s[key] === value) return s;
    const next = { ...s };
    if (value === undefined) delete next[key];
    else next[key] = value;
    return next;
  });
}

export function readContext(): ContextSnapshot {
  return contextStore.state;
}

export function useWhen(expr: WhenExpr | undefined): boolean {
  return useSelector(contextStore, (s) => evaluateWhen(expr, s));
}

export function resetContext(): void {
  contextStore.setState(() => ({}));
}
