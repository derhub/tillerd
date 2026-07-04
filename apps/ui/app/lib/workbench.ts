import { useSelector } from "@tanstack/react-store";
import React from "react";

import { setGlobalSetting, settingsStore } from "~/lib/settings/context";
import {
  WORKBENCH_DEFAULTS,
  WORKBENCH_PANEL_SIZE_KEY,
  WORKBENCH_PANEL_TAB_KEY,
  WORKBENCH_PANEL_VISIBLE_KEY,
  WORKBENCH_PREFIX,
  WORKBENCH_SIDEBAR_SIZE_KEY,
  WORKBENCH_SIDEBAR_VISIBLE_KEY,
  WORKBENCH_VIEW_KEY,
} from "~/lib/settings/keys";

// Reactive, settings-backed workbench layout state. Every value is a global-scope
// settings key (see settings/keys), so the whole chrome layout restores on launch
// and converges across windows through the settings broadcast. Reads subscribe to
// the settings store; writes are fire-and-forget through setGlobalSetting.

function useBoolSetting(key: string, fallback: boolean): boolean {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  return typeof raw === "boolean" ? raw : fallback;
}

function useNumberSetting(key: string, fallback: number): number {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  return typeof raw === "number" ? raw : fallback;
}

function useStringSetting(key: string, fallback: string): string {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  return typeof raw === "string" && raw ? raw : fallback;
}

export function useWorkbenchView(): readonly [string, (id: string) => void] {
  const view = useStringSetting(WORKBENCH_VIEW_KEY, WORKBENCH_DEFAULTS.view);
  const set = React.useCallback((id: string) => setGlobalSetting(WORKBENCH_VIEW_KEY, id), []);
  return [view, set] as const;
}

export function useSidebarVisible(): readonly [boolean, (visible: boolean) => void] {
  const visible = useBoolSetting(WORKBENCH_SIDEBAR_VISIBLE_KEY, WORKBENCH_DEFAULTS.sidebarVisible);
  const set = React.useCallback(
    (v: boolean) => setGlobalSetting(WORKBENCH_SIDEBAR_VISIBLE_KEY, v),
    [],
  );
  return [visible, set] as const;
}

export function useSidebarSize(): readonly [number, (px: number) => void] {
  const size = useNumberSetting(WORKBENCH_SIDEBAR_SIZE_KEY, WORKBENCH_DEFAULTS.sidebarSize);
  const set = React.useCallback(
    (px: number) => setGlobalSetting(WORKBENCH_SIDEBAR_SIZE_KEY, px),
    [],
  );
  return [size, set] as const;
}

export function useBottomPanelVisible(): readonly [boolean, (visible: boolean) => void] {
  const visible = useBoolSetting(WORKBENCH_PANEL_VISIBLE_KEY, WORKBENCH_DEFAULTS.panelVisible);
  const set = React.useCallback(
    (v: boolean) => setGlobalSetting(WORKBENCH_PANEL_VISIBLE_KEY, v),
    [],
  );
  return [visible, set] as const;
}

export function useBottomPanelSize(): readonly [number, (px: number) => void] {
  const size = useNumberSetting(WORKBENCH_PANEL_SIZE_KEY, WORKBENCH_DEFAULTS.panelSize);
  const set = React.useCallback((px: number) => setGlobalSetting(WORKBENCH_PANEL_SIZE_KEY, px), []);
  return [size, set] as const;
}

export function useBottomPanelTab(): readonly [string, (tab: string) => void] {
  const tab = useStringSetting(WORKBENCH_PANEL_TAB_KEY, WORKBENCH_DEFAULTS.panelTab);
  const set = React.useCallback((t: string) => setGlobalSetting(WORKBENCH_PANEL_TAB_KEY, t), []);
  return [tab, set] as const;
}

// Imperative setters for non-hook call sites (command handlers, the status-bar bell).
export function setWorkbenchView(id: string): void {
  setGlobalSetting(WORKBENCH_VIEW_KEY, id);
}

export function setBottomPanelVisible(visible: boolean): void {
  setGlobalSetting(WORKBENCH_PANEL_VISIBLE_KEY, visible);
}

export function setBottomPanelTab(tab: string): void {
  setGlobalSetting(WORKBENCH_PANEL_TAB_KEY, tab);
}

// Reveal the bottom panel on a specific tab -- the affordance the status-bar bell and
// health "logs" links use to jump straight to their content (spec: tab-specific opener).
export function showBottomPanelTab(tab: string): void {
  setBottomPanelTab(tab);
  setBottomPanelVisible(true);
}

// Test/reset hygiene: drop the persisted workbench keys without a wire write.
export function resetWorkbench(): void {
  settingsStore.setState((s) => {
    const values = { ...s.values };
    for (const key of Object.keys(values)) {
      if (key.startsWith(WORKBENCH_PREFIX)) delete values[key];
    }
    return { ...s, values };
  });
}
