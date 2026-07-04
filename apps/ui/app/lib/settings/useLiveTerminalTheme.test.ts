import type { SettingView } from "@tillerd/client-bindings";
import type { Terminal } from "@xterm/xterm";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { delegatingQuery } from "~/lib/test/real-bindings";

// Shared by both terminal-pane hosts (desktop and web): a scheme change must reach every
// mounted terminal's live `options.theme` without a respawn. Guards the exact regression the
// settings-terminal-scheme e2e spec caught -- a per-component setting read that silently
// diverged between the two hosts.

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
const { useLiveTerminalTheme } = await import("./useLiveTerminalTheme");

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

describe("useLiveTerminalTheme", () => {
  test("a scheme change mirrors onto the mounted terminal's live theme option", async () => {
    const term = { options: {} } as Terminal;
    const termRef = { current: term };

    const { result } = renderHook(() => useLiveTerminalTheme(termRef), { wrapper });

    await waitFor(() => expect(result.current.background).toBe("#0d1117")); // github-dark default

    act(() => setGlobalSetting("terminal.scheme", "github-light"));

    await waitFor(() => expect(result.current.background).toBe("#ffffff"));
    // The whole point of the hook: the mutation must land on the actual Terminal instance,
    // not just the returned theme value a consumer might forget to apply.
    expect(term.options.theme?.background).toBe("#ffffff");
  });

  test("a scheme change before the terminal mounts is a no-op, not a crash", async () => {
    const termRef = { current: null as Terminal | null };
    const { result } = renderHook(() => useLiveTerminalTheme(termRef), { wrapper });

    await waitFor(() => expect(result.current.background).toBe("#0d1117"));
    expect(() => act(() => setGlobalSetting("terminal.scheme", "github-light"))).not.toThrow();
    await waitFor(() => expect(result.current.background).toBe("#ffffff"));
  });
});
