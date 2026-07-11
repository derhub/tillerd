import type { SettingView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from "bun:test";

import { delegatingQuery } from "~/lib/test/real-bindings";

// Ported from the retired popover's coverage: selecting an appearance must apply immediately
// (document class) and persist to the durable settings store -- unchanged behavior, new host.

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];
let active = false;

beforeEach(() => {
  active = true;
});

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
}));

const { SettingsProvider, _resetForTests } = await import("~/lib/settings/context");
const { AppearanceSection } = await import("./AppearanceSection");

afterEach(() => {
  cleanup();
  active = false;
  document.documentElement.classList.remove("dark");
  localStorage.clear();
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
      <AppearanceSection />
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

describe("AppearanceSection", () => {
  test("selecting dark toggles the document class immediately and persists it", async () => {
    renderSection({ theme: "light" });

    const trigger = await screen.findByLabelText("Theme");
    await waitFor(() => expect(trigger.textContent).toContain("light"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("dark");
    pointerActivate(option);

    expect(document.documentElement.classList.contains("dark")).toBe(true);
    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "theme", valueJson: JSON.stringify("dark") }),
      ),
    );
  });
});
