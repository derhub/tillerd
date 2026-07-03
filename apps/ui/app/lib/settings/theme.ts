import { DEFAULT_THEME, isTheme, type Theme } from "./keys";

export const THEME_CACHE_KEY = "tillerd.theme";

export function applyTheme(
  root: { classList: { toggle(token: string, force: boolean): void } },
  theme: Theme,
): void {
  root.classList.toggle("dark", theme === "dark");
}

export function readCachedTheme(storage: Pick<Storage, "getItem">): Theme {
  const cached = storage.getItem(THEME_CACHE_KEY);
  return isTheme(cached) ? cached : DEFAULT_THEME;
}

export function writeCachedTheme(storage: Pick<Storage, "setItem">, theme: Theme): void {
  storage.setItem(THEME_CACHE_KEY, theme);
}

// Runs raw in the document head before the bundle loads; must stay self-contained.
export const THEME_PAINT_SCRIPT = `(function(){try{var t=localStorage.getItem(${JSON.stringify(
  THEME_CACHE_KEY,
)});document.documentElement.classList.toggle("dark",t!=="light");}catch(e){}})();`;
