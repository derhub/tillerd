import type { SettingView } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, expect, mock, test } from "bun:test";

import { delegatingQuery } from "~/lib/test/real-bindings";

const settingSetCalls: { scope: string; projectId: null; key: string; valueJson: string }[] = [];
let failSettingSet = false;
let active = false;

beforeEach(() => {
  active = true;
});

// Spread the real module so non-overridden exports stay intact: mock.module is process-global
// and persists across files, so a partial replacement would clobber sibling suites that use the
// real query/command wrappers. query() delegates unowned keys to the captured realQuery so sibling
// suites keep their real query()/whenReady() path under any file order.
const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: (key: string, args: any) => {
    if (!active) return actualBindings.runCommand(key, args);
    if (key === "settingSet") settingSetCalls.push(args);
    if (failSettingSet) return Promise.reject(new Error("store unavailable"));
    return Promise.resolve(null) as any;
  },
  query: delegatingQuery({ settingList: () => ({ queryFn: async () => [] }) }, () => active),
  getQueryClient: () => {
    if (!active) return actualBindings.getQueryClient();
    return {
      ensureQueryData: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
      fetchQuery: (opts: { queryFn: () => Promise<unknown> }) => opts.queryFn(),
      getQueryData: () => undefined,
      invalidateQueries: () => Promise.resolve(),
    } as any;
  },
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
  active = false;
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

test("rapid writes to one key reach the wire in order, intermediates coalesced", async () => {
  await hydrateSettings(() => Promise.resolve([]));

  setGlobalSetting("terminal.scheme", "a");
  setGlobalSetting("terminal.scheme", "b");
  setGlobalSetting("terminal.scheme", "c");

  await waitFor(() => {
    const values = settingSetCalls
      .filter((call) => call.key === "terminal.scheme")
      .map((call) => JSON.parse(call.valueJson));
    // One write on the wire at a time: "b" never sends, "c" follows "a".
    expect(values).toEqual(["a", "c"]);
  });
  expect(settingsStore.state.values["terminal.scheme"]).toBe("c");
});

test("a queued newer value still reaches the wire after a failed write", async () => {
  await hydrateSettings(() => Promise.resolve([]));
  failSettingSet = true;

  setGlobalSetting("terminal.scheme", "first-fails");
  setGlobalSetting("terminal.scheme", "must-still-send");

  await waitFor(() => {
    const values = settingSetCalls
      .filter((call) => call.key === "terminal.scheme")
      .map((call) => JSON.parse(call.valueJson));
    expect(values).toEqual(["first-fails", "must-still-send"]);
  });
});

test("a failed pointer write never blocks the interaction", async () => {
  await hydrateSettings(() => Promise.resolve([]));
  failSettingSet = true;

  setGlobalSetting("view.active-workspace", "ws-2");

  // The in-memory value updates immediately; the rejected persist is swallowed
  // (the orchestrator records reachable failures -- the renderer never records).
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

test("hydration with no persisted theme applies the dark default to the document", async () => {
  await hydrateSettings(() => Promise.resolve([]));

  expect(document.documentElement.classList.contains("dark")).toBe(true);
  expect(localStorage.getItem(THEME_CACHE_KEY)).toBe("dark");
});

test("a pinned startup workspace overrides the restored last-active pointer at launch", async () => {
  await hydrateSettings(() =>
    Promise.resolve(
      listFrom({
        "view.active-workspace": "ws-last-used",
        "general.startupWorkspace": "ws-pinned",
      }),
    ),
  );

  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-pinned");
});

test("an unset startup workspace leaves the last-active pointer untouched", async () => {
  await hydrateSettings(() =>
    Promise.resolve(listFrom({ "view.active-workspace": "ws-last-used" })),
  );

  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-last-used");
});

test("a later rehydrate does not re-pin the startup workspace mid-session", async () => {
  const list = () =>
    Promise.resolve(
      listFrom({
        "view.active-workspace": "ws-last-used",
        "general.startupWorkspace": "ws-pinned",
      }),
    );

  // First hydration (launch) applies the override once.
  await hydrateSettings(list);
  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-pinned");

  // A mid-session rehydrate (a profile activation re-runs hydrateSettings) must not force the
  // pinned workspace back on -- the window stays on whatever the refreshed snapshot reports.
  await hydrateSettings(list);
  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-last-used");
});
