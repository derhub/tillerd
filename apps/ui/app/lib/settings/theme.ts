import { DEFAULT_THEME, isTheme, type Theme } from "./keys";

/** localStorage key holding the paint-time theme cache (read before first paint in root). */
export const THEME_CACHE_KEY = "tillerd.theme";

/**
 * Apply a theme to the document root by toggling the `.dark` class. The token sheets key
 * dark styles off `.dark` (see `app.css`); its absence is the light appearance.
 */
export function applyTheme(
  root: { classList: { toggle(token: string, force: boolean): void } },
  theme: Theme,
): void {
  root.classList.toggle("dark", theme === "dark");
}

/** Read the cached theme for paint-time application. Defaults to {@link DEFAULT_THEME}. */
export function readCachedTheme(storage: Pick<Storage, "getItem">): Theme {
  const cached = storage.getItem(THEME_CACHE_KEY);
  return isTheme(cached) ? cached : DEFAULT_THEME;
}

/** Persist the paint-time theme cache. */
export function writeCachedTheme(storage: Pick<Storage, "setItem">, theme: Theme): void {
  storage.setItem(THEME_CACHE_KEY, theme);
}

/**
 * Inline script (stringified) injected into the document head so the cached theme applies
 * before first paint, with no flash. Mirrors {@link applyTheme} + {@link readCachedTheme}
 * but must stay self-contained (it runs raw, before the bundle loads).
 */
export const THEME_PAINT_SCRIPT = `(function(){try{var t=localStorage.getItem(${JSON.stringify(
  THEME_CACHE_KEY,
)});document.documentElement.classList.toggle("dark",t!=="light");}catch(e){}})();`;
