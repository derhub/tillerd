import type { ReactNode } from "react";

import { Store, useSelector } from "@tanstack/react-store";
import React from "react";

import { loadSettingsSource, type SettingsSource } from "~/lib/transport/settings-source";

import { DEFAULT_THEME, THEME_KEY, isTheme, type Theme } from "./keys";
import { applyTheme, readCachedTheme, writeCachedTheme } from "./theme";

// TanStack Store owns shared client UI state. Server data stays in the Query cache, never here.
interface SettingsState {
  values: Record<string, unknown>;
  source: SettingsSource | null;
}

export const settingsStore = new Store<SettingsState>({ values: {}, source: null });

// Writes that fire before hydration resolves the source. Without this buffer the
// fire-and-forget persist below is dropped against a null source, so an early
// preset/theme change never reaches the orchestrator and is lost on reload.
let pendingWrites: { key: string; value: unknown }[] = [];

export async function hydrateSettings(
  resolve: () => Promise<SettingsSource | null> = loadSettingsSource,
): Promise<void> {
  const source = await resolve();
  settingsStore.setState((s) => ({ ...s, source }));
  const pending = pendingWrites;
  pendingWrites = [];
  if (!source) return;
  for (const w of pending) void source.setSetting({ scope: "global", key: w.key, value: w.value });
  const entries = await source.listSettings({ scope: "global" });
  const values: Record<string, unknown> = {};
  for (const e of entries) values[e.key] = e.value;
  // Pre-hydration changes win over the listed snapshot so they are not reverted.
  for (const w of pending) values[w.key] = w.value;
  settingsStore.setState((s) => ({ ...s, values }));
  if (isTheme(values[THEME_KEY])) {
    applyTheme(document.documentElement, values[THEME_KEY]);
    writeCachedTheme(localStorage, values[THEME_KEY]);
  }
}

export function setGlobalSetting(key: string, value: unknown): void {
  const { source } = settingsStore.state;
  settingsStore.setState((s) => ({ ...s, values: { ...s.values, [key]: value } }));
  if (source) void source.setSetting({ scope: "global", key, value });
  else pendingWrites.push({ key, value });
}

export function SettingsProvider({
  children,
  resolve = loadSettingsSource,
}: {
  children: ReactNode;
  resolve?: () => Promise<SettingsSource | null>;
}) {
  React.useEffect(() => {
    void hydrateSettings(resolve);
  }, [resolve]);
  return <>{children}</>;
}

export function useGlobalSetting(
  key: string,
  fallback: string,
): { value: string; setValue: (value: string) => void } {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  const value = typeof raw === "string" ? raw : fallback;
  const setValue = React.useCallback((next: string) => setGlobalSetting(key, next), [key]);
  return { value, setValue };
}

export function useTheme(): { theme: Theme; setTheme: (theme: Theme) => void } {
  // Paint cache (read once) is the pre-hydration fallback; durable value wins once loaded.
  const cachedFallback = React.useMemo(
    () => (typeof localStorage === "undefined" ? DEFAULT_THEME : readCachedTheme(localStorage)),
    [],
  );
  const raw = useSelector(settingsStore, (s) => s.values[THEME_KEY]);
  const theme: Theme = isTheme(raw) ? raw : cachedFallback;

  const setTheme = React.useCallback((next: Theme) => {
    applyTheme(document.documentElement, next);
    writeCachedTheme(localStorage, next);
    setGlobalSetting(THEME_KEY, next);
  }, []);

  return { theme, setTheme };
}
