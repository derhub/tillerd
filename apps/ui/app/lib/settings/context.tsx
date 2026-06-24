import type { ReactNode } from "react";
import type { SettingView } from "@tillerd/client-bindings";

import { Store, useSelector } from "@tanstack/react-store";
import React from "react";

import { commands, ensureResult } from "@tillerd/client-bindings";

import { DEFAULT_THEME, THEME_KEY, isTheme, type Theme } from "./keys";
import { applyTheme, readCachedTheme, writeCachedTheme } from "./theme";

// TanStack Store owns shared client UI state. Server data stays in the Query cache, never here.
interface SettingsState {
  values: Record<string, unknown>;
}

export const settingsStore = new Store<SettingsState>({ values: {} });

// Writes that fire before hydration resolves the source. Without this buffer the
// fire-and-forget persist below is dropped against a null source, so an early
// preset/theme change never reaches the orchestrator and is lost on reload.
let pendingWrites: { key: string; value: unknown }[] = [];
let hydrated = false;

// Test-only: reset module-level state between tests.
export function _resetForTests(): void {
  pendingWrites = [];
  hydrated = false;
}

function persist(key: string, value: unknown): void {
  void commands
    .settingSet({ scope: "global", projectId: null, key, valueJson: JSON.stringify(value) })
    .then(ensureResult);
}

function defaultResolve(): Promise<SettingView[]> {
  return commands.settingList({ scope: "global", projectId: null }).then(ensureResult);
}

export async function hydrateSettings(
  resolve: () => Promise<SettingView[]> = defaultResolve,
): Promise<void> {
  const entries = await resolve();
  const pending = pendingWrites;
  pendingWrites = [];
  hydrated = true;
  for (const w of pending) persist(w.key, w.value);
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
  settingsStore.setState((s) => ({ ...s, values: { ...s.values, [key]: value } }));
  if (hydrated) persist(key, value);
  else pendingWrites.push({ key, value });
}

export function SettingsProvider({
  children,
  resolve = defaultResolve,
}: {
  children: ReactNode;
  resolve?: () => Promise<SettingView[]>;
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
