import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, test } from "bun:test";

import type { SettingsSource } from "~/lib/transport/settings-source";

import {
  SettingsProvider,
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
  settingsStore.setState(() => ({ values: {}, source: null }));
});

function fakeSource(initial: Record<string, unknown> = {}): {
  source: SettingsSource;
  writes: { key: string; value: unknown }[];
} {
  const store = new Map(Object.entries(initial));
  const writes: { key: string; value: unknown }[] = [];
  const source: SettingsSource = {
    getSetting: async ({ key }) => store.get(key) ?? null,
    setSetting: async ({ key, value }) => {
      store.set(key, value);
      writes.push({ key, value });
    },
    listSettings: async () => [...store.entries()].map(([key, value]) => ({ key, value })),
  };
  return { source, writes };
}

function wrapperFor(source: SettingsSource | null) {
  return ({ children }: { children: ReactNode }) => (
    <SettingsProvider resolve={() => Promise.resolve(source)}>{children}</SettingsProvider>
  );
}

test("a setting change propagates live to every consumer of the same key", async () => {
  const { source } = fakeSource({ "terminal.scheme": "github-dark" });
  const { result } = renderHook(
    () => ({
      panel: useGlobalSetting("terminal.scheme", "github-dark"),
      terminal: useGlobalSetting("terminal.scheme", "github-dark"),
    }),
    { wrapper: wrapperFor(source) },
  );

  await waitFor(() => expect(result.current.terminal.value).toBe("github-dark"));

  act(() => result.current.panel.setValue("github-light"));
  expect(result.current.terminal.value).toBe("github-light");
});

test("useGlobalSetting hydrates from the source and falls back before then", async () => {
  const { source } = fakeSource({ "terminal.scheme": "github-light" });
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor(source),
  });
  await waitFor(() => expect(result.current.value).toBe("github-light"));
});

test("a write fired before hydration reaches the source once it resolves", async () => {
  const { source, writes } = fakeSource();

  setGlobalSetting("keybindings.preset", "vscode");
  expect(writes).toHaveLength(0);

  await hydrateSettings(() => Promise.resolve(source));

  expect(writes).toContainEqual({ key: "keybindings.preset", value: "vscode" });
  expect(settingsStore.state.values["keybindings.preset"]).toBe("vscode");
});

test("useGlobalSetting uses the fallback with no source (off the desktop host)", async () => {
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor(null),
  });
  expect(result.current.value).toBe("github-dark");
});

test("setTheme applies the class, caches it, and persists to the source", async () => {
  const { source, writes } = fakeSource({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(source) });
  await waitFor(() => expect(result.current.theme).toBe("light"));

  act(() => result.current.setTheme("dark"));

  expect(result.current.theme).toBe("dark");
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("dark");
  await waitFor(() => expect(writes).toContainEqual({ key: "theme", value: "dark" }));
});

test("the provider applies the hydrated durable theme to the document", async () => {
  const { source } = fakeSource({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(source) });

  await waitFor(() => expect(result.current.theme).toBe("light"));
  expect(document.documentElement.classList.contains("dark")).toBe(false);
});
