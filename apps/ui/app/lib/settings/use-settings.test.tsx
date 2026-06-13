import { afterEach, expect, test } from "bun:test";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";

import type { SettingsSource } from "~/lib/transport/settings-source";
import { THEME_CACHE_KEY } from "./theme";
import { useStringSetting, useTheme } from "./use-settings";

afterEach(() => {
  cleanup();
  localStorage.clear();
  document.documentElement.classList.remove("dark");
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
    listSettings: async () => [],
  };
  return { source, writes };
}

test("useTheme hydrates the durable value and applies it to the document", async () => {
  const { source } = fakeSource({ theme: "light" });
  const { result } = renderHook(() => useTheme(source));

  await waitFor(() => expect(result.current.theme).toBe("light"));
  expect(document.documentElement.classList.contains("dark")).toBe(false);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("light");
});

test("setTheme applies the class, caches it, and persists to the source", async () => {
  const { source, writes } = fakeSource();
  const { result } = renderHook(() => useTheme(source));

  act(() => result.current.setTheme("light"));

  expect(result.current.theme).toBe("light");
  expect(document.documentElement.classList.contains("dark")).toBe(false);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("light");
  await waitFor(() => expect(writes).toContainEqual({ key: "theme", value: "light" }));
});

test("useStringSetting hydrates the stored value and persists writes", async () => {
  const { source, writes } = fakeSource({ "terminal.scheme": "github-light" });
  const { result } = renderHook(() => useStringSetting(source, "terminal.scheme", "github-dark"));

  await waitFor(() => expect(result.current.value).toBe("github-light"));

  act(() => result.current.setValue("github-dark"));
  expect(result.current.value).toBe("github-dark");
  await waitFor(() =>
    expect(writes).toContainEqual({ key: "terminal.scheme", value: "github-dark" }),
  );
});

test("useStringSetting falls back to the default with no source", () => {
  const { result } = renderHook(() => useStringSetting(null, "terminal.scheme", "github-dark"));
  expect(result.current.value).toBe("github-dark");
});
