import type { SettingView } from "@tillerd/client-bindings";
import type { Terminal } from "@xterm/xterm";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { delegatingQuery } from "~/lib/test/real-bindings";

// Mirrors the terminal-scheme live path (useLiveTerminalTheme): a typography/buffer setting change
// must reach every mounted terminal's live `options.*` without a respawn, refitting after the
// change so the PTY relearns the new geometry.

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: () => Promise.resolve(null),
  query: delegatingQuery({ settingList: () => ({ queryFn: async () => [] }) }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    fetchQuery: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    getQueryData: () => undefined,
    invalidateQueries: () => Promise.resolve(),
  }),
}));

const { SettingsProvider, _resetForTests, setGlobalSetting } = await import("./context");
const { useLiveTerminalTypography } = await import("./useLiveTerminalTypography");

afterEach(() => {
  cleanup();
  _resetForTests();
});

afterAll(() => mock.restore());

function wrapper({ children }: { children: ReactNode }) {
  return React.createElement(
    SettingsProvider,
    { resolve: () => Promise.resolve([] as SettingView[]) },
    children,
  );
}

describe("useLiveTerminalTypography", () => {
  test("returns the hardcoded defaults when no settings are stored", async () => {
    const termRef = { current: null as Terminal | null };
    const { result } = renderHook(() => useLiveTerminalTypography(termRef), { wrapper });
    await waitFor(() => expect(result.current.fontSize).toBe(13));
    expect(result.current.cursorStyle).toBe("block");
    expect(result.current.cursorBlink).toBe(true);
    expect(result.current.scrollback).toBe(1000);
  });

  test("a font-size change lands on the mounted terminal and triggers a refit", async () => {
    const term = { options: {} } as Terminal;
    const termRef = { current: term };
    let refits = 0;
    const { result } = renderHook(() => useLiveTerminalTypography(termRef, () => (refits += 1)), {
      wrapper,
    });
    await waitFor(() => expect(result.current.fontSize).toBe(13));
    const before = refits;

    act(() => setGlobalSetting("terminal.fontSize", 18));

    await waitFor(() => expect(term.options.fontSize).toBe(18));
    expect(result.current.fontSize).toBe(18);
    expect(refits).toBeGreaterThan(before);
  });

  test("a cursor-style change mirrors onto the live options", async () => {
    const term = { options: {} } as Terminal;
    const termRef = { current: term };
    const { result } = renderHook(() => useLiveTerminalTypography(termRef), { wrapper });
    await waitFor(() => expect(result.current.cursorStyle).toBe("block"));

    act(() => setGlobalSetting("terminal.cursorStyle", "bar"));

    await waitFor(() => expect(term.options.cursorStyle).toBe("bar"));
  });

  test("a change before the terminal mounts is a no-op, not a crash", async () => {
    const termRef = { current: null as Terminal | null };
    const { result } = renderHook(() => useLiveTerminalTypography(termRef), { wrapper });
    await waitFor(() => expect(result.current.fontSize).toBe(13));
    expect(() => act(() => setGlobalSetting("terminal.fontSize", 20))).not.toThrow();
    await waitFor(() => expect(result.current.fontSize).toBe(20));
  });
});
