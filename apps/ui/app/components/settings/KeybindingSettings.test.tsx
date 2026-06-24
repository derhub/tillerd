import type { SettingView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { ACTION } from "~/lib/commands/ids";
import { _resetForTests, SettingsProvider } from "~/lib/settings/context";

import { KeybindingSettings } from "./KeybindingSettings";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];

mock.module("@tillerd/client-bindings", () => ({
  commands: {
    settingSet: (args: { scope: string; projectId: null; key: string; valueJson: string }) => {
      settingSetCalls.push(args);
      return Promise.resolve({ status: "ok", data: null });
    },
    settingList: () => Promise.resolve({ status: "ok", data: [] }),
    settingGet: () => Promise.resolve({ status: "ok", data: null }),
  },
  ensureResult: (r: { status: string; data: unknown } | { status: string; error: unknown }) => {
    if (r.status === "ok") return (r as { status: string; data: unknown }).data;
    throw new Error(String((r as { status: string; error: unknown }).error));
  },
}));

afterEach(() => {
  cleanup();
  _resetForTests();
  settingSetCalls.length = 0;
});

function listFrom(initial: Record<string, unknown>): SettingView[] {
  return Object.entries(initial).map(([key, value]) => ({ key, value }));
}

function renderPanel(initial: Record<string, unknown> = {}) {
  const list = listFrom(initial);
  render(
    <SettingsProvider resolve={() => Promise.resolve(list)}>
      <KeybindingSettings />
    </SettingsProvider>,
  );
  return settingSetCalls;
}

describe("KeybindingSettings", () => {
  test("selecting a preset persists it", async () => {
    const sets = renderPanel();
    const select = (await screen.findByLabelText("Keybinding preset")) as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "vscode" } });
    await waitFor(() =>
      expect(
        sets.find((s) => s.key === "keybindings.preset"),
      ).toEqual(
        expect.objectContaining({ key: "keybindings.preset", valueJson: JSON.stringify("vscode") }),
      ),
    );
  });

  test("editing an override persists the canonicalized chord", async () => {
    renderPanel();
    const input = (await screen.findByTestId(`kb-${ACTION.surfaceClose}`)) as HTMLInputElement;
    // The default binding hydrates into the input; edit only once it has settled.
    await waitFor(() => expect(input.value).toBe("CmdOrCtrl+W"));
    fireEvent.change(input, { target: { value: "cmd+shift+w" } });
    fireEvent.blur(input);
    await waitFor(() => {
      const written = settingSetCalls.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      // valueJson is JSON.stringify(value) where value is itself the JSON-encoded overrides string.
      expect(JSON.parse(JSON.parse(written!.valueJson))[ACTION.surfaceClose]).toBe("CmdOrCtrl+Shift+W");
    });
  });

  test("clearing an override removes it", async () => {
    renderPanel({
      "keybindings.overrides": JSON.stringify({ [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W" }),
    });
    const input = (await screen.findByTestId(`kb-${ACTION.surfaceClose}`)) as HTMLInputElement;
    // Wait for the seeded override to hydrate so a late reset cannot clobber the cleared draft.
    await waitFor(() => expect(input.value).toBe("CmdOrCtrl+Shift+W"));
    fireEvent.change(input, { target: { value: "" } });
    fireEvent.blur(input);
    await waitFor(() => {
      const written = settingSetCalls.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      expect(ACTION.surfaceClose in JSON.parse(JSON.parse(written!.valueJson))).toBe(false);
    });
  });
});
