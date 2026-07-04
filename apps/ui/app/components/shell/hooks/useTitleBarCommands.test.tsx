import { act, cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";
import React from "react";

import { resetContext } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, useCommands, type Command } from "~/lib/commands/registry";
import { resetUiStore } from "~/lib/store";

import { useTitleBarCommands } from "./useTitleBarCommands";

afterEach(() => {
  cleanup();
  resetContext();
  resetUiStore();
});

function Harness({ onCommands }: { onCommands: (commands: Command[]) => void }) {
  useTitleBarCommands();
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
  return {
    find: (id: string) => commands.find((c) => c.id === id),
  };
}

describe("useTitleBarCommands", () => {
  test("panelToggleLeft starts checked (left panel visible by default) and toggles the store", () => {
    const { find } = mount();
    expect(find(ACTION.panelToggleLeft)?.checked).toBe(true);

    void act(() => find(ACTION.panelToggleLeft)?.run());

    expect(find(ACTION.panelToggleLeft)?.checked).toBe(false);
  });

  test("panelToggleRight starts unchecked (hidden by default) and toggles the store", () => {
    const { find } = mount();
    expect(find(ACTION.panelToggleRight)?.checked).toBe(false);

    void act(() => find(ACTION.panelToggleRight)?.run());

    expect(find(ACTION.panelToggleRight)?.checked).toBe(true);
  });

  test("panelToggleBottom starts unchecked (hidden by default) and toggles the store", () => {
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
