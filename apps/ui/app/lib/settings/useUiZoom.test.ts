import type { SettingView } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { delegatingQuery } from "~/lib/test/real-bindings";

// Guards the ui-settings-editor "Zoom applies live" scenario: a zoom change (local or
// restored from disk) must reach the actual webview via setWebviewZoom, not just the
// returned value a consumer might forget to apply.

let active = false;
beforeEach(() => {
  active = true;
});

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: (key: string, args: any) => {
    if (!active) return actualBindings.runCommand(key, args);
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

// Spread the real module so unrelated exports (listenEvent, currentWindow, ...) stay
// intact for sibling suites -- mock.module is process-global (see context.test.tsx).
const actualTauriEvents = await import("~/lib/tauriEvents");
const zoomCalls: number[] = [];
void mock.module("~/lib/tauriEvents", () => ({
  ...actualTauriEvents,
  setWebviewZoom: (scaleFactor: number) => {
    if (!active) return actualTauriEvents.setWebviewZoom(scaleFactor);
    zoomCalls.push(scaleFactor);
    return Promise.resolve();
  },
}));

const { SettingsProvider, _resetForTests, setGlobalSetting } = await import("./context");
const { useUiZoom } = await import("./useUiZoom");
const { DEFAULT_UI_ZOOM, UI_ZOOM_KEY, UI_ZOOM_MAX, UI_ZOOM_MIN } = await import("./keys");

afterEach(() => {
  cleanup();
  active = false;
  _resetForTests();
  zoomCalls.length = 0;
});

afterAll(() => mock.restore());

function wrapper({ children }: { children: ReactNode }) {
  return React.createElement(
    SettingsProvider,
    { resolve: () => Promise.resolve([] as SettingView[]) },
    children,
  );
}

describe("useUiZoom", () => {
  test("defaults to 1x and applies it to the webview on mount", async () => {
    const { result } = renderHook(() => useUiZoom(), { wrapper });

    await waitFor(() => expect(result.current.zoom).toBe(DEFAULT_UI_ZOOM));
    await waitFor(() => expect(zoomCalls).toContain(DEFAULT_UI_ZOOM));
  });

  test("a zoom change mirrors onto the webview via setWebviewZoom", async () => {
    const { result } = renderHook(() => useUiZoom(), { wrapper });
    await waitFor(() => expect(result.current.zoom).toBe(DEFAULT_UI_ZOOM));

    act(() => setGlobalSetting(UI_ZOOM_KEY, 1.2));

    await waitFor(() => expect(result.current.zoom).toBe(1.2));
    expect(zoomCalls).toContain(1.2);
  });

  test("setZoom clamps to the configured bounds and persists the clamped value", async () => {
    const { result } = renderHook(() => useUiZoom(), { wrapper });
    await waitFor(() => expect(result.current.zoom).toBe(DEFAULT_UI_ZOOM));

    act(() => result.current.setZoom(UI_ZOOM_MAX + 1));
    await waitFor(() => expect(result.current.zoom).toBe(UI_ZOOM_MAX));

    act(() => result.current.setZoom(UI_ZOOM_MIN - 1));
    await waitFor(() => expect(result.current.zoom).toBe(UI_ZOOM_MIN));
  });

  test("reset restores the default zoom", async () => {
    const { result } = renderHook(() => useUiZoom(), { wrapper });
    await waitFor(() => expect(result.current.zoom).toBe(DEFAULT_UI_ZOOM));

    act(() => result.current.setZoom(1.5));
    await waitFor(() => expect(result.current.zoom).toBe(1.5));

    act(() => result.current.reset());
    await waitFor(() => expect(result.current.zoom).toBe(DEFAULT_UI_ZOOM));
  });
});
