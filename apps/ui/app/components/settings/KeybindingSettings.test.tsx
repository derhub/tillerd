import type { SettingView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";

import { ACTION } from "~/lib/commands/ids";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];

// Spread the real module so non-overridden exports stay intact: mock.module is process-global
// and persists across files, so a partial replacement would clobber sibling suites that use the
// real query/command wrappers. afterAll restores so this override does not leak past this file.
const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: (
    key: string,
    args: { scope: string; projectId: null; key: string; valueJson: string },
  ) => {
    if (key === "settingSet") settingSetCalls.push(args);
    return Promise.resolve(null);
  },
  query: () => ({ queryFn: async () => [] }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
  }),
}));

const { _resetForTests, SettingsProvider } = await import("~/lib/settings/context");
const { KeybindingSettings } = await import("./KeybindingSettings");

afterEach(() => {
  cleanup();
  _resetForTests();
  settingSetCalls.length = 0;
});

afterAll(() => mock.restore());

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
      expect(sets.find((s) => s.key === "keybindings.preset")).toEqual(
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
      expect(JSON.parse(JSON.parse(written!.valueJson))[ACTION.surfaceClose]).toBe(
        "CmdOrCtrl+Shift+W",
      );
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
