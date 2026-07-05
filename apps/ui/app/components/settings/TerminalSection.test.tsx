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

  test("changing the font size persists a number on blur", async () => {
    renderSection({ "terminal.fontSize": 13 });

    const input = await screen.findByLabelText("Terminal font size");
    fireEvent.change(input, { target: { value: "20" } });
    fireEvent.blur(input);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "terminal.fontSize", valueJson: JSON.stringify(20) }),
      ),
    );
  });

  test("a numeric field does not persist mid-edit, only on commit", async () => {
    renderSection({ "terminal.fontSize": 13 });

    const input = await screen.findByLabelText("Terminal font size");
    // Typing the leading digit of a larger value must not reach the store: a per-keystroke
    // commit would push font size 1 to every mounted terminal (unreadable flash).
    fireEvent.change(input, { target: { value: "1" } });
    expect(settingSetCalls).toHaveLength(0);
  });

  test("an out-of-range line height is clamped on commit, never persisted raw", async () => {
    renderSection({ "terminal.lineHeight": 2 });

    const input = await screen.findByLabelText("Terminal line height");
    // xterm throws for lineHeight < 1; the field must clamp to the floor before it reaches the
    // store rather than persist 0.5 and crash every terminal mount.
    fireEvent.change(input, { target: { value: "0.5" } });
    fireEvent.blur(input);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "terminal.lineHeight", valueJson: JSON.stringify(1) }),
      ),
    );
    expect(
      settingSetCalls.some(
        (c) => c.key === "terminal.lineHeight" && c.valueJson === JSON.stringify(0.5),
      ),
    ).toBe(false);
  });

  test("selecting a cursor style persists it", async () => {
    renderSection({ "terminal.cursorStyle": "block" });

    const trigger = await screen.findByLabelText("Terminal cursor style");
    await waitFor(() => expect(trigger.textContent).toContain("block"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("bar");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "terminal.cursorStyle", valueJson: JSON.stringify("bar") }),
      ),
    );
  });

  test("toggling copy-on-select persists the boolean", async () => {
    renderSection({ "terminal.copyOnSelect": false });

    // The switch's accessible name is carried by its own role="switch" element (a wrapping
    // <label> also associates the hidden native checkbox it renders alongside, so
    // findByLabelText would match both -- getByRole targets the one with the click handler).
    const toggle = await screen.findByRole("switch", { name: "Copy on select" });
    pointerActivate(toggle);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({ key: "terminal.copyOnSelect", valueJson: JSON.stringify(true) }),
      ),
    );
  });
});
