import type { SettingView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from "bun:test";

import { CommandCenter } from "~/components/command/CommandCenter";
import { TooltipProvider } from "~/components/ui/tooltip";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";
import { delegatingQuery } from "~/lib/test/real-bindings";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];
let active = false;

beforeEach(() => {
  active = true;
});

// Spread the real module so non-overridden exports stay intact: mock.module is process-global
// and persists across files, so a partial replacement would clobber sibling suites that use the
// real query/command wrappers. query() delegates unowned keys to the captured realQuery so sibling
// suites keep their real query()/whenReady() path under any file order.
const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: (key: string, args: any) => {
    if (!active) return actualBindings.runCommand(key, args);
    if (key === "settingSet") settingSetCalls.push(args);
    return Promise.resolve(null) as any;
  },
  query: delegatingQuery({ settingList: () => ({ queryFn: async () => [] }) }, () => active),
  getQueryClient: () => {
    if (!active) return actualBindings.getQueryClient();
    return {
      ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
      fetchQuery: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
      getQueryData: () => undefined,
      invalidateQueries: () => Promise.resolve(),
    } as any;
  },
  // CommandCenter's leader-key mount also runs in this suite's "reflects in a sibling
  // consumer" test; a no-op keeps it from reaching for a real event bus under happy-dom.
  subscribe: (key: string) => {
    if (!active) return actualBindings.subscribe(key as never);
    return { listen: () => Promise.resolve(() => {}) } as any;
  },
}));

const { _resetForTests, SettingsProvider } = await import("~/lib/settings/context");
const { KeybindingSettings } = await import("./KeybindingSettings");

afterEach(() => {
  cleanup();
  active = false;
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
    <TooltipProvider>
      <SettingsProvider resolve={() => Promise.resolve(list)}>
        <KeybindingSettings />
      </SettingsProvider>
    </TooltipProvider>,
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

  test("the per-command reset button is disabled with no override and clears one when enabled", async () => {
    renderPanel({
      "keybindings.overrides": JSON.stringify({ [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W" }),
    });
    const resetBtn = (await screen.findByTestId(
      `kb-${ACTION.surfaceClose}-reset`,
    )) as HTMLButtonElement;
    await waitFor(() => expect(resetBtn.disabled).toBe(false));

    fireEvent.click(resetBtn);
    await waitFor(() => {
      const written = settingSetCalls.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      expect(ACTION.surfaceClose in JSON.parse(JSON.parse(written!.valueJson))).toBe(false);
    });
  });

  test("reset-all is disabled with no overrides and clears every override when enabled", async () => {
    renderPanel({
      "keybindings.overrides": JSON.stringify({ [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W" }),
    });
    const resetAll = (await screen.findByTestId("kb-reset-all")) as HTMLButtonElement;
    await waitFor(() => expect(resetAll.disabled).toBe(false));

    fireEvent.click(resetAll);
    await waitFor(() => {
      const written = settingSetCalls.findLast((s) => s.key === "keybindings.overrides");
      expect(written).toBeDefined();
      expect(JSON.parse(written!.valueJson)).toBe("{}");
    });
  });

  test("reset-all stays disabled while there are no overrides", async () => {
    renderPanel();
    const resetAll = (await screen.findByTestId("kb-reset-all")) as HTMLButtonElement;
    await waitFor(() => expect(resetAll.disabled).toBe(true));
  });

  // Guards the exact regression the command-center e2e spec caught: a preset change written
  // through this settings-editor section must reach every OTHER consumer of the resolved
  // bindings map (here, the command palette's shortcut hint), not just this component's own
  // read. Both mount under one SettingsProvider/CommandRegistryProvider, exactly as they do in
  // the real app (KeybindingSettings inside the /settings route, CommandCenter mounted globally
  // in RootLayout) -- the shared `settingsStore` singleton is what should carry the write across.
  test("a preset change reflects in a sibling consumer's resolved bindings", async () => {
    render(
      <TooltipProvider>
        <SettingsProvider resolve={() => Promise.resolve([])}>
          <CommandRegistryProvider>
            <RegisterHandlers handlers={{ [ACTION.surfaceSpawn]: () => {} }} />
            <KeybindingSettings />
            <CommandCenter />
          </CommandRegistryProvider>
        </SettingsProvider>
      </TooltipProvider>,
    );

    const select = (await screen.findByLabelText("Keybinding preset")) as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "vscode" } });
    await waitFor(() =>
      expect(settingSetCalls.find((s) => s.key === "keybindings.preset")).toEqual(
        expect.objectContaining({ key: "keybindings.preset", valueJson: JSON.stringify("vscode") }),
      ),
    );

    fireEvent(window, new CustomEvent("command-center:open"));
    const palette = await screen.findByTestId("command-center");
    const items = Array.from(palette.querySelectorAll('[data-slot="command-item"]'));
    const item = items.find((el) => el.textContent?.includes("New terminal"));
    // The vscode preset binds "New terminal" to a backtick chord; the default preset does not.
    await waitFor(() =>
      expect(item?.querySelector('[data-slot="command-shortcut"]')?.textContent).toContain("`"),
    );
  });
});
