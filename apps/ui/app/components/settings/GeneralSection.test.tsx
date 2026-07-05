import type { SettingView } from "@tillerd/client-bindings";

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";

import { makeQueryClient } from "~/lib/queryClient";
import { delegatingQuery } from "~/lib/test/real-bindings";

// General settings (ui-settings-editor "General settings"): the close-confirmation toggle
// mirrors PANEL_CLOSE_CONFIRM_SKIP_KEY (see PanelContent's own consumer of the same key), and
// the startup workspace picker writes a plain global setting consumed once at launch by
// context.tsx's hydrateSettings override -- this suite only proves the write, not the launch
// wiring (covered in context.test.tsx).

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];

const workspaces = [
  { id: "ws-1", name: "Default", status: "active", pinned: false },
  { id: "ws-2", name: "Work", status: "active", pinned: false },
];

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
  query: delegatingQuery({
    settingList: () => ({ queryFn: async () => [] }),
    workspaceList: () => ({
      queryKey: ["workspaces", "list", null],
      queryFn: async () => workspaces,
    }),
  }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    fetchQuery: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    getQueryData: () => undefined,
    invalidateQueries: () => Promise.resolve(),
  }),
}));

const { SettingsProvider, _resetForTests } = await import("~/lib/settings/context");
const { GeneralSection } = await import("./GeneralSection");

afterEach(() => {
  cleanup();
  _resetForTests();
  settingSetCalls.length = 0;
});

afterAll(() => mock.restore());

function listFrom(initial: Record<string, unknown>): SettingView[] {
  return Object.entries(initial).map(([key, value]) => ({ key, value }));
}

function renderSection(initial: Record<string, unknown> = {}) {
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <SettingsProvider resolve={() => Promise.resolve(listFrom(initial))}>
        <GeneralSection />
      </SettingsProvider>
    </QueryClientProvider>,
  );
}

// base-ui's Select reacts to Floating UI's useClick, which distinguishes a real pointer click
// from a synthetic one by the pointerdown that precedes it -- see TerminalSection.test.tsx.
function pointerActivate(el: HTMLElement): void {
  fireEvent.pointerDown(el, { button: 0 });
  fireEvent.mouseDown(el, { button: 0 });
  fireEvent.pointerUp(el, { button: 0 });
  fireEvent.mouseUp(el, { button: 0 });
  fireEvent.click(el);
}

describe("GeneralSection", () => {
  test("turning on the close-confirmation switch clears the skip flag", async () => {
    renderSection({ "panel.closeConfirm.skip": true });

    const toggle = await screen.findByRole("switch", {
      name: "Confirm before closing a running terminal",
    });
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));

    pointerActivate(toggle);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({
          key: "panel.closeConfirm.skip",
          valueJson: JSON.stringify(false),
        }),
      ),
    );
  });

  test("selecting a startup workspace persists its id", async () => {
    renderSection();

    const trigger = await screen.findByLabelText("Startup workspace");
    await waitFor(() => expect(trigger.textContent).toContain("Last used"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("Work");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({
          key: "general.startupWorkspace",
          valueJson: JSON.stringify("ws-2"),
        }),
      ),
    );
  });

  test("the zoom control shows the current level as a percentage", async () => {
    renderSection({ "ui.zoom": 1.5 });

    await waitFor(() => expect(screen.getByTestId("ui-zoom-value").textContent).toBe("150%"));
  });

  test("zooming in persists a larger zoom level", async () => {
    renderSection({ "ui.zoom": 1 });

    const zoomIn = await screen.findByLabelText("Zoom in");
    fireEvent.click(zoomIn);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "ui.zoom", valueJson: JSON.stringify(1.1) }),
      ),
    );
  });
});
