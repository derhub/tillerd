import { act, cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { resetContext } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, useCommands, type Command } from "~/lib/commands/registry";
import { _resetForTests, settingsStore } from "~/lib/settings/context";
import { resetUiStore } from "~/lib/store";

import { useWorkbenchCommands } from "./useWorkbenchCommands";

beforeEach(() => {
  _resetForTests();
  settingsStore.setState(() => ({ values: {} }));
  resetUiStore();
  resetContext();
});

afterEach(() => {
  cleanup();
  resetUiStore();
  settingsStore.setState(() => ({ values: {} }));
  _resetForTests();
  resetContext();
});

function Harness({ onCommands }: { onCommands: (commands: Command[]) => void }) {
  useWorkbenchCommands();
  onCommands(useCommands());
  return null;
}

function mount() {
  let commands: Command[] = [];
  render(
    <CommandRegistryProvider>
      <Harness onCommands={(c) => (commands = c)} />
    </CommandRegistryProvider>,
  );
  return { find: (id: string) => commands.find((c) => c.id === id) };
}

describe("useWorkbenchCommands", () => {
  test("the Sessions view is active by default and the others are not", () => {
    const { find } = mount();
    expect(find(ACTION.viewSessions)?.checked).toBe(true);
    expect(find(ACTION.viewCommands)?.checked).toBe(false);
  });

  test("activating another view marks it active", () => {
    const { find } = mount();
    void act(() => find(ACTION.viewCommands)?.run());
    expect(find(ACTION.viewCommands)?.checked).toBe(true);
    expect(find(ACTION.viewSessions)?.checked).toBe(false);
  });

  test("panelToggleLeft starts checked (sidebar visible by default) and toggles", () => {
    const { find } = mount();
    expect(find(ACTION.panelToggleLeft)?.checked).toBe(true);
    void act(() => find(ACTION.panelToggleLeft)?.run());
    expect(find(ACTION.panelToggleLeft)?.checked).toBe(false);
  });

  test("activating the already-active view toggles the sidebar", () => {
    const { find } = mount();
    expect(find(ACTION.panelToggleLeft)?.checked).toBe(true);
    void act(() => find(ACTION.viewSessions)?.run());
    expect(find(ACTION.panelToggleLeft)?.checked).toBe(false);
  });

  test("panelToggleBottom starts unchecked (hidden by default) and toggles", () => {
    const { find } = mount();
    expect(find(ACTION.panelToggleBottom)?.checked).toBe(false);
    void act(() => find(ACTION.panelToggleBottom)?.run());
    expect(find(ACTION.panelToggleBottom)?.checked).toBe(true);
  });

  test("commandToggle flips commandCenterOpen and its checked state follows", () => {
    const { find } = mount();
    expect(find(ACTION.commandToggle)?.checked).toBe(false);
    void act(() => find(ACTION.commandToggle)?.run());
    expect(find(ACTION.commandToggle)?.checked).toBe(true);
    void act(() => find(ACTION.commandToggle)?.run());
    expect(find(ACTION.commandToggle)?.checked).toBe(false);
  });
});
