import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { resetContext, setContextKey } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";

import { TitleBar } from "./TitleBar";

afterEach(() => {
  cleanup();
  resetContext();
});

function renderBar(handlers: Record<string, () => void>) {
  return render(
    <CommandRegistryProvider>
      <RegisterHandlers handlers={handlers} />
      <TitleBar />
    </CommandRegistryProvider>,
  );
}

describe("TitleBar toolbar", () => {
  test("renders a button for each active titlebar command", () => {
    renderBar({ [ACTION.panelToggleLeft]: () => {} });
    expect(screen.queryByRole("button", { name: "Toggle sidebar" })).not.toBeNull();
  });

  test("omits a titlebar command with no registered handler", () => {
    renderBar({});
    expect(screen.queryByRole("button", { name: "Toggle sidebar" })).toBeNull();
  });

  test("a toggle button reflects its checked state from context", () => {
    setContextKey("sidebarVisible", true);
    renderBar({ [ACTION.panelToggleLeft]: () => {} });
    expect(
      screen.getByRole("button", { name: "Toggle sidebar" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  test("clicking a toggle button runs its command", () => {
    const spy = mock(() => {});
    renderBar({ [ACTION.panelToggleLeft]: spy });
    fireEvent.click(screen.getByRole("button", { name: "Toggle sidebar" }));
    expect(spy).toHaveBeenCalledTimes(1);
  });

  test("renders no window-control buttons (the OS draws native controls)", () => {
    renderBar({ [ACTION.panelToggleLeft]: () => {} });
    expect(screen.queryByRole("button", { name: /minimize|maximize|close/i })).toBeNull();
  });
});
