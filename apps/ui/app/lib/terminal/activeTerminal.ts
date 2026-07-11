import { Store } from "@tanstack/react-store";

import type { CommandHandler } from "~/lib/commands/types";

import { ACTION } from "~/lib/commands/ids";

// Imperative surface of the focused terminal pane. A single set of command handlers (registered
// once in the panel host) acts on whichever pane is focused, so palette and context-menu commands
// never need per-pane registration -- the pane publishes its controller here on focus.
export interface TerminalController {
  openFind(initialQuery?: string): void;
  copySelection(): void;
  paste(): void;
  selectAll(): void;
  clear(): void;
  searchSelection(): void;
}

export const activeTerminalStore = new Store<{ controller: TerminalController | null }>({
  controller: null,
});

export function setActiveTerminal(controller: TerminalController): void {
  activeTerminalStore.setState(() => ({ controller }));
}

// Clear only if still current: a later focus may have already handed off to another pane.
export function clearActiveTerminal(controller: TerminalController): void {
  activeTerminalStore.setState((s) => (s.controller === controller ? { controller: null } : s));
}

function runOnActive(fn: (c: TerminalController) => void): void {
  const c = activeTerminalStore.state.controller;
  if (c) fn(c);
}

// Stable module constant: safe to hand straight to RegisterHandlers without memoization.
export const terminalCommandHandlers: Record<string, CommandHandler> = {
  [ACTION.terminalFind]: () => runOnActive((c) => c.openFind()),
  [ACTION.terminalCopy]: () => runOnActive((c) => c.copySelection()),
  [ACTION.terminalPaste]: () => runOnActive((c) => c.paste()),
  [ACTION.terminalSelectAll]: () => runOnActive((c) => c.selectAll()),
  [ACTION.terminalClear]: () => runOnActive((c) => c.clear()),
  [ACTION.terminalSearchSelection]: () => runOnActive((c) => c.searchSelection()),
};
