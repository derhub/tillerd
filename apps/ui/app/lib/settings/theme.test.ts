import { expect, test } from "bun:test";

import {
  THEME_CACHE_KEY,
  THEME_PAINT_SCRIPT,
  applyTheme,
  readCachedTheme,
  writeCachedTheme,
} from "./theme";

function fakeRoot() {
  const classes = new Set<string>();
  return {
    classList: {
      toggle(token: string, force: boolean) {
        if (force) classes.add(token);
        else classes.delete(token);
      },
    },
    has: (token: string) => classes.has(token),
  };
}

function fakeStorage(initial: Record<string, string> = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
  };
}

test("applyTheme adds the dark class for dark and removes it for light", () => {
  const root = fakeRoot();
  applyTheme(root, "dark");
  expect(root.has("dark")).toBe(true);
  applyTheme(root, "light");
  expect(root.has("dark")).toBe(false);
});

test("readCachedTheme defaults to dark when unset or invalid", () => {
  expect(readCachedTheme(fakeStorage())).toBe("dark");
  expect(readCachedTheme(fakeStorage({ [THEME_CACHE_KEY]: "bogus" }))).toBe("dark");
});

test("readCachedTheme returns a stored light value", () => {
  expect(readCachedTheme(fakeStorage({ [THEME_CACHE_KEY]: "light" }))).toBe("light");
});

test("writeCachedTheme persists the value under the cache key", () => {
  const storage = fakeStorage();
  writeCachedTheme(storage, "light");
  expect(storage.getItem(THEME_CACHE_KEY)).toBe("light");
});

test("the paint script references the cache key and toggles the dark class", () => {
  expect(THEME_PAINT_SCRIPT).toContain(THEME_CACHE_KEY);
  expect(THEME_PAINT_SCRIPT).toContain('classList.toggle("dark"');
});
