import type { SettingView } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { QueryObserver } from "@tanstack/react-query";
import { Store, useSelector } from "@tanstack/react-store";
import { getQueryClient, query, runCommand } from "@tillerd/client-bindings";
import React from "react";

import { broadcastInvalidate } from "../crossWindowSync";
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

// Fire-and-forget: the interaction that caused the write never blocks on it. A
// failure that reaches the orchestrator is recorded there (`command-error`) and
// pushed back over the notification channel -- the renderer never records. On
// success sibling windows converge via the invalidation broadcast (the local
// store is already updated synchronously by setGlobalSetting).
function persist(key: string, value: unknown): void {
  runCommand("settingSet", {
    scope: "global",
    projectId: null,
    key,
    valueJson: JSON.stringify(value),
  }).then(
    () => {
      // Converge the local query cache too (the broadcast skips its own window).
      // Optional call: bootstrap-test stubs provide only ensureQueryData.
      void getQueryClient().invalidateQueries?.({ queryKey: ["settings"] });
      broadcastInvalidate([["settings"]]);
    },
    () => {},
  );
}

function defaultResolve(): Promise<SettingView[]> {
  return getQueryClient().ensureQueryData(
    query("settingList", { scope: "global", projectId: null }),
  );
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

// Keep the store converged with the settings query after hydration: when a sibling
// window's write invalidates ["settings"] (cross-window broadcast) the refetch lands
// here and re-seeds the store, so pointer changes appear live in every window.
export function watchSettings(): () => void {
  const client = getQueryClient();
  // Bootstrap-test stubs provide only ensureQueryData; live convergence needs a
  // full QueryClient, so degrade to a no-op rather than throw.
  if (typeof (client as { getQueryCache?: unknown }).getQueryCache !== "function") {
    return () => {};
  }
  const observer = new QueryObserver(
    client,
    query("settingList", { scope: "global", projectId: null }),
  );
  return observer.subscribe((result) => {
    if (!hydrated || !result.data) return;
    const values: Record<string, unknown> = {};
    for (const e of result.data) values[e.key] = e.value;
    settingsStore.setState((s) => ({ ...s, values }));
  });
}

// Plain helper (not a hook): hydrate, then watch; returns a cleanup that also
// cancels an in-flight hydration's watch handoff.
function startSettings(resolve: () => Promise<SettingView[]>): () => void {
  let unwatch: (() => void) | undefined;
  let disposed = false;
  void hydrateSettings(resolve).then(() => {
    if (!disposed) unwatch = watchSettings();
  });
  return () => {
    disposed = true;
    unwatch?.();
  };
}

export function SettingsProvider({
  children,
  resolve = defaultResolve,
}: {
  children: ReactNode;
  resolve?: () => Promise<SettingView[]>;
}) {
  React.useEffect(() => startSettings(resolve), [resolve]);
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
