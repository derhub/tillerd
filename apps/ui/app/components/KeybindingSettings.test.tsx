/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

import { KeybindingSettings } from "./KeybindingSettings";
import { ACTION } from "~/lib/commands/ids";
import { SettingsProvider } from "~/lib/settings/context";
import type { SettingsSource } from "~/lib/transport/settings-source";

afterEach(cleanup);

function makeSource(initial: Record<string, unknown> = {}) {
  const values: Record<string, unknown> = { ...initial };
  const sets: { key: string; value: unknown }[] = [];
  const source = {
    listSettings: async () => Object.entries(values).map(([key, value]) => ({ key, value })),
    setSetting: async ({ key, value }: { key: string; value: unknown }) => {
      values[key] = value;
      sets.push({ key, value });
    },
  } as unknown as SettingsSource;
  return { source, sets };
}

function renderPanel(initial: Record<string, unknown> = {}) {
  const { source, sets } = makeSource(initial);
  render(
    <SettingsProvider resolve={async () => source}>
      <KeybindingSettings />
    </SettingsProvider>,
  );
  return sets;
}

describe("KeybindingSettings", () => {
  test("selecting a preset persists it", async () => {
    const sets = renderPanel();
    const select = (await screen.findByLabelText("Keybinding preset")) as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "vscode" } });
    await waitFor(() =>
      expect(sets.find((s) => s.key === "keybindings.preset")?.value).toBe("vscode"),
    );
  });

  test("editing an override persists the canonicalized chord", async () => {
    const sets = renderPanel();
    const input = (await screen.findByTestId(`kb-${ACTION.surfaceClose}`)) as HTMLInputElement;
    // The default binding hydrates into the input; edit only once it has settled.
    await waitFor(() => expect(input.value).toBe("CmdOrCtrl+W"));
    fireEvent.change(input, { target: { value: "cmd+shift+w" } });
    fireEvent.blur(input);
    await waitFor(() => {
      const written = sets.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      expect(JSON.parse(written!.value as string)[ACTION.surfaceClose]).toBe("CmdOrCtrl+Shift+W");
    });
  });

  test("clearing an override removes it", async () => {
    const sets = renderPanel({
      "keybindings.overrides": JSON.stringify({ [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W" }),
    });
    const input = (await screen.findByTestId(`kb-${ACTION.surfaceClose}`)) as HTMLInputElement;
    // Wait for the seeded override to hydrate so a late reset cannot clobber the cleared draft.
    await waitFor(() => expect(input.value).toBe("CmdOrCtrl+Shift+W"));
    fireEvent.change(input, { target: { value: "" } });
    fireEvent.blur(input);
    await waitFor(() => {
      const written = sets.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      expect(ACTION.surfaceClose in JSON.parse(written!.value as string)).toBe(false);
    });
  });
});
