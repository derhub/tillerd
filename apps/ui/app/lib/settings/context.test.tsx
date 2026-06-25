import type { ReactNode } from "react";
import type { SettingView } from "@tillerd/client-bindings";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, mock, test } from "bun:test";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];

void mock.module("@tillerd/client-bindings", () => ({
  runCommand: (key: string, args: { scope: string; projectId: null; key: string; valueJson: string }) => {
    if (key === "settingSet") settingSetCalls.push(args);
    return Promise.resolve(null);
  },
  query: () => ({ queryFn: async () => [] }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
  }),
}));

import {
  SettingsProvider,
  _resetForTests,
  hydrateSettings,
  setGlobalSetting,
  settingsStore,
  useGlobalSetting,
  useTheme,
} from "./context";
import { THEME_CACHE_KEY } from "./theme";

afterEach(() => {
  cleanup();
  localStorage.clear();
  document.documentElement.classList.remove("dark");
  settingsStore.setState(() => ({ values: {} }));
  _resetForTests();
  settingSetCalls.length = 0;
});

function listFrom(initial: Record<string, unknown>): SettingView[] {
  return Object.entries(initial).map(([key, value]) => ({ key, value }));
}

function wrapperFor(list: SettingView[]) {
  return ({ children }: { children: ReactNode }) => (
    <SettingsProvider resolve={() => Promise.resolve(list)}>{children}</SettingsProvider>
  );
}

test("a setting change propagates live to every consumer of the same key", async () => {
  const list = listFrom({ "terminal.scheme": "github-dark" });
  const { result } = renderHook(
    () => ({
      panel: useGlobalSetting("terminal.scheme", "github-dark"),
      terminal: useGlobalSetting("terminal.scheme", "github-dark"),
    }),
    { wrapper: wrapperFor(list) },
  );

  await waitFor(() => expect(result.current.terminal.value).toBe("github-dark"));

  act(() => result.current.panel.setValue("github-light"));
  expect(result.current.terminal.value).toBe("github-light");
});

test("useGlobalSetting hydrates from the source and falls back before then", async () => {
  const list = listFrom({ "terminal.scheme": "github-light" });
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor(list),
  });
  await waitFor(() => expect(result.current.value).toBe("github-light"));
});

test("a write fired before hydration reaches the source once it resolves", async () => {
  setGlobalSetting("keybindings.preset", "vscode");
  expect(settingSetCalls).toHaveLength(0);

  await hydrateSettings(() => Promise.resolve([]));

  expect(settingSetCalls).toContainEqual(
    expect.objectContaining({ key: "keybindings.preset", valueJson: JSON.stringify("vscode") }),
  );
  expect(settingsStore.state.values["keybindings.preset"]).toBe("vscode");
});

test("useGlobalSetting uses the fallback with no source (off the desktop host)", async () => {
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor([]),
  });
  expect(result.current.value).toBe("github-dark");
});

test("setTheme applies the class, caches it, and persists to the source", async () => {
  const list = listFrom({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(list) });
  await waitFor(() => expect(result.current.theme).toBe("light"));

  act(() => result.current.setTheme("dark"));

  expect(result.current.theme).toBe("dark");
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("dark");
  await waitFor(() =>
    expect(settingSetCalls).toContainEqual(
      expect.objectContaining({ key: "theme", valueJson: JSON.stringify("dark") }),
    ),
  );
});

test("the provider applies the hydrated durable theme to the document", async () => {
  const list = listFrom({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(list) });

  await waitFor(() => expect(result.current.theme).toBe("light"));
  expect(document.documentElement.classList.contains("dark")).toBe(false);
});
