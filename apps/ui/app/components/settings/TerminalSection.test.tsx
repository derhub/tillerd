import type { SettingView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";

import { delegatingQuery } from "~/lib/test/real-bindings";

// Ported from the retired popover's coverage: selecting a terminal scheme persists to the
// shared settings store, which live-retheme mounted terminals -- unchanged behavior, new host.

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];

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
  query: delegatingQuery({ settingList: () => ({ queryFn: async () => [] }) }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    fetchQuery: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    getQueryData: () => undefined,
    invalidateQueries: () => Promise.resolve(),
  }),
}));

const { SettingsProvider, _resetForTests } = await import("~/lib/settings/context");
const { TerminalSection } = await import("./TerminalSection");

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
    <SettingsProvider resolve={() => Promise.resolve(listFrom(initial))}>
      <TerminalSection />
    </SettingsProvider>,
  );
}

// base-ui's Select (trigger open AND item pick) reacts to Floating UI's useClick, which
// distinguishes a real pointer click from a synthetic one by the pointerdown that precedes it --
// fireEvent.click alone (a bare "click" with no preceding pointerdown) is a no-op on either.
function pointerActivate(el: HTMLElement): void {
  fireEvent.pointerDown(el, { button: 0 });
  fireEvent.mouseDown(el, { button: 0 });
  fireEvent.pointerUp(el, { button: 0 });
  fireEvent.mouseUp(el, { button: 0 });
  fireEvent.click(el);
}

describe("TerminalSection", () => {
  test("selecting a scheme persists it", async () => {
    renderSection({ "terminal.scheme": "github-dark" });

    const trigger = await screen.findByLabelText("Terminal scheme");
    await waitFor(() => expect(trigger.textContent).toContain("github-dark"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("github-light");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({
          key: "terminal.scheme",
          valueJson: JSON.stringify("github-light"),
        }),
      ),
    );
  });
});
