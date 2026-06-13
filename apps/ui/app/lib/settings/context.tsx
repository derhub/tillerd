import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { loadSettingsSource, type SettingsSource } from "~/lib/transport/settings-source";
import { DEFAULT_THEME, THEME_KEY, isTheme, type Theme } from "./keys";
import { applyTheme, readCachedTheme, writeCachedTheme } from "./theme";

interface SettingsState {
  /** Current global settings values, keyed by setting key. */
  values: Record<string, unknown>;
  /** Update one global setting: shared state + durable persistence. */
  setValue: (key: string, value: unknown) => void;
}

const SettingsStateContext = createContext<SettingsState | null>(null);

/**
 * Single reactive source of truth for global settings. Hydrates every global value once (one
 * `listSettings` call), so all consumers — the panel and every terminal — read the same state
 * and re-render together when a value changes. `null` source (off the desktop host) degrades to
 * defaults. Inject `resolve` in tests.
 */
export function SettingsProvider({
  children,
  resolve = loadSettingsSource,
}: {
  children: ReactNode;
  resolve?: () => Promise<SettingsSource | null>;
}) {
  const [source, setSource] = useState<SettingsSource | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const s = await resolve();
      if (cancelled) return;
      setSource(s);
      if (!s) return;
      const entries = await s.listSettings({ scope: "global" });
      if (cancelled) return;
      const map: Record<string, unknown> = {};
      for (const e of entries) map[e.key] = e.value;
      setValues(map);
      // Reconcile the durable theme with the paint-time cache.
      if (isTheme(map[THEME_KEY])) {
        applyTheme(document.documentElement, map[THEME_KEY]);
        writeCachedTheme(localStorage, map[THEME_KEY]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [resolve]);

  const setValue = useCallback(
    (key: string, value: unknown) => {
      setValues((prev) => ({ ...prev, [key]: value }));
      void source?.setSetting({ scope: "global", key, value });
    },
    [source],
  );

  const state = useMemo<SettingsState>(() => ({ values, setValue }), [values, setValue]);
  return <SettingsStateContext value={state}>{children}</SettingsStateContext>;
}

function useSettingsState(): SettingsState | null {
  return useContext(SettingsStateContext);
}

/**
 * A single global setting as reactive shared state: every consumer of the same key re-renders
 * when it changes. Falls back to `fallback` until hydrated / off the desktop host.
 */
export function useGlobalSetting(
  key: string,
  fallback: string,
): { value: string; setValue: (value: string) => void } {
  const state = useSettingsState();
  const raw = state?.values[key];
  const value = typeof raw === "string" ? raw : fallback;
  // Depend on `setValue` (stable once the source resolves), not the whole `state` (new ref on
  // every write), so an unrelated setting change doesn't recreate this consumer's setter.
  const setter = state?.setValue;
  const setValue = useCallback((next: string) => setter?.(key, next), [setter, key]);
  return { value, setValue };
}

/** Theme as reactive shared state, applied to the document root and the paint-time cache on change. */
export function useTheme(): { theme: Theme; setTheme: (theme: Theme) => void } {
  const state = useSettingsState();
  // The paint cache (read once) is the pre-hydration fallback; the durable value wins once loaded.
  const cachedFallback = useMemo(
    () => (typeof localStorage === "undefined" ? DEFAULT_THEME : readCachedTheme(localStorage)),
    [],
  );
  const raw = state?.values[THEME_KEY];
  const theme: Theme = isTheme(raw) ? raw : cachedFallback;

  const setter = state?.setValue;
  const setTheme = useCallback(
    (next: Theme) => {
      applyTheme(document.documentElement, next);
      writeCachedTheme(localStorage, next);
      setter?.(THEME_KEY, next);
    },
    [setter],
  );

  return { theme, setTheme };
}
