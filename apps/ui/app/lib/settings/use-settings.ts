import { useCallback, useEffect, useState } from "react";

import { loadSettingsSource, type SettingsSource } from "~/lib/transport/settings-source";
import { DEFAULT_THEME, THEME_KEY, isTheme, type Theme } from "./keys";
import { applyTheme, readCachedTheme, writeCachedTheme } from "./theme";

/** Lazily resolve the host settings source once. `null` off the desktop host. */
export function useSettingsSource(
  resolve: () => Promise<SettingsSource | null> = loadSettingsSource,
): SettingsSource | null {
  const [source, setSource] = useState<SettingsSource | null>(null);
  useEffect(() => {
    let cancelled = false;
    void resolve().then((s) => {
      if (!cancelled) setSource(s);
    });
    return () => {
      cancelled = true;
    };
  }, [resolve]);
  return source;
}

/**
 * Theme state backed by the durable settings store with a localStorage paint cache.
 * Initial value comes from the cache (already applied before paint by the root script);
 * the durable value hydrates on mount and reconciles the cache.
 */
export function useTheme(source: SettingsSource | null): {
  theme: Theme;
  setTheme: (theme: Theme) => void;
} {
  const [theme, setThemeState] = useState<Theme>(() =>
    typeof localStorage === "undefined" ? DEFAULT_THEME : readCachedTheme(localStorage),
  );

  useEffect(() => {
    if (!source) return;
    let cancelled = false;
    void source.getSetting({ scope: "global", key: THEME_KEY }).then((value) => {
      if (cancelled || !isTheme(value)) return;
      setThemeState(value);
      applyTheme(document.documentElement, value);
      writeCachedTheme(localStorage, value);
    });
    return () => {
      cancelled = true;
    };
  }, [source]);

  const setTheme = useCallback(
    (next: Theme) => {
      setThemeState(next);
      applyTheme(document.documentElement, next);
      writeCachedTheme(localStorage, next);
      void source?.setSetting({ scope: "global", key: THEME_KEY, value: next });
    },
    [source],
  );

  return { theme, setTheme };
}

/**
 * A single global string setting with a default. Hydrates from the durable store on mount;
 * writes persist immediately. Used for the terminal scheme, default command/template, and
 * sidebar expand state.
 */
export function useStringSetting(
  source: SettingsSource | null,
  key: string,
  fallback: string,
): { value: string; setValue: (value: string) => void } {
  const [value, setValueState] = useState(fallback);

  useEffect(() => {
    if (!source) return;
    let cancelled = false;
    void source.getSetting({ scope: "global", key }).then((v) => {
      if (!cancelled && typeof v === "string") setValueState(v);
    });
    return () => {
      cancelled = true;
    };
  }, [source, key]);

  const setValue = useCallback(
    (next: string) => {
      setValueState(next);
      void source?.setSetting({ scope: "global", key, value: next });
    },
    [source, key],
  );

  return { value, setValue };
}
