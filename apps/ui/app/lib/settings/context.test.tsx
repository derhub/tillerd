import type { SettingView } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterAll, afterEach, expect, mock, test } from "bun:test";

import { delegatingQuery } from "~/lib/test/real-bindings";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];
let failSettingSet = false;

// Spread the real module so non-overridden exports stay intact: mock.module is process-global
// and persists across files, so a partial replacement would clobber sibling suites that use the
// real query/command wrappers. query() delegates unowned keys to the captured realQuery so sibling
// suites keep their real query()/whenReady() path under any file order.
const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: (
    key: string,
    args: { scope: string; projectId: null; key: string; valueJson: string },
  ) => {
    if (key === "settingSet") settingSetCalls.push(args);
    if (failSettingSet) return Promise.reject(new Error("store unavailable"));
    return Promise.resolve(null);
  },
  query: delegatingQuery({ settingList: () => ({ queryFn: async () => [] }) }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
    // No getQueryCache on purpose: watchSettings degrades to a no-op under this stub.
    invalidateQueries: () => Promise.resolve(),
  }),
}));

const {
  SettingsProvider,
  _resetForTests,
  hydrateSettings,
  setGlobalSetting,
  settingsStore,
  useGlobalSetting,
  useTheme,
} = await import("./context");
const { THEME_CACHE_KEY } = await import("./theme");

afterEach(() => {
  cleanup();
  localStorage.clear();
  document.documentElement.classList.remove("dark");
  settingsStore.setState(() => ({ values: {} }));
  _resetForTests();
  settingSetCalls.length = 0;
  failSettingSet = false;
});

afterAll(() => mock.restore());

function listFrom(initial: Record<string, unknown>): SettingView[] {
  return Object.entries(initial).map(([key, value]) => ({ key, value }));
}

function wrapperFor(list: SettingView[]) {
  return ({ children }: { children: ReactNode }) => (
    <SettingsProvider resolve={() => Promise.resolve(list)}>{children}</SettingsProvider>
  );
}

test("a setting change propagates live to every consumer of the same key", async () => {
  const list = listFrom({ "terminal.scheme": "github-dark" });
  const { result } = renderHook(
    () => ({
      panel: useGlobalSetting("terminal.scheme", "github-dark"),
      terminal: useGlobalSetting("terminal.scheme", "github-dark"),
    }),
    { wrapper: wrapperFor(list) },
  );

  await waitFor(() => expect(result.current.terminal.value).toBe("github-dark"));

  act(() => result.current.panel.setValue("github-light"));
  expect(result.current.terminal.value).toBe("github-light");
});

test("useGlobalSetting hydrates from the source and falls back before then", async () => {
  const list = listFrom({ "terminal.scheme": "github-light" });
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor(list),
  });
  await waitFor(() => expect(result.current.value).toBe("github-light"));
});

test("a write fired before hydration reaches the source once it resolves", async () => {
  setGlobalSetting("keybindings.preset", "vscode");
  expect(settingSetCalls).toHaveLength(0);

  await hydrateSettings(() => Promise.resolve([]));

  expect(settingSetCalls).toContainEqual(
    expect.objectContaining({ key: "keybindings.preset", valueJson: JSON.stringify("vscode") }),
  );
  expect(settingsStore.state.values["keybindings.preset"]).toBe("vscode");
});

test("useGlobalSetting uses the fallback with no source (off the desktop host)", async () => {
  const { result } = renderHook(() => useGlobalSetting("terminal.scheme", "github-dark"), {
    wrapper: wrapperFor([]),
  });
  expect(result.current.value).toBe("github-dark");
});

test("setTheme applies the class, caches it, and persists to the source", async () => {
  const list = listFrom({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(list) });
  await waitFor(() => expect(result.current.theme).toBe("light"));

  act(() => result.current.setTheme("dark"));

  expect(result.current.theme).toBe("dark");
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("dark");
  await waitFor(() =>
    expect(settingSetCalls).toContainEqual(
      expect.objectContaining({ key: "theme", valueJson: JSON.stringify("dark") }),
    ),
  );
});

test("view pointers restore from the settings source on hydration", async () => {
  await hydrateSettings(() =>
    Promise.resolve(
      listFrom({
        "view.active-workspace": "ws-9",
        "sidebar.expanded.p-1": true,
        "view.last-session.p-1": "s-4",
      }),
    ),
  );

  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-9");
  expect(settingsStore.state.values["sidebar.expanded.p-1"]).toBe(true);
  expect(settingsStore.state.values["view.last-session.p-1"]).toBe("s-4");
});

test("a failed pointer write never blocks the interaction", async () => {
  await hydrateSettings(() => Promise.resolve([]));
  failSettingSet = true;

  setGlobalSetting("view.active-workspace", "ws-2");

  // The in-memory value updates immediately; the rejected persist is swallowed
  // (the orchestrator records reachable failures — the renderer never records).
  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-2");
  await waitFor(() =>
    expect(settingSetCalls).toContainEqual(
      expect.objectContaining({ key: "view.active-workspace" }),
    ),
  );
});

test("the provider applies the hydrated durable theme to the document", async () => {
  const list = listFrom({ theme: "light" });
  const { result } = renderHook(() => useTheme(), { wrapper: wrapperFor(list) });

  await waitFor(() => expect(result.current.theme).toBe("light"));
  expect(document.documentElement.classList.contains("dark")).toBe(false);
});
