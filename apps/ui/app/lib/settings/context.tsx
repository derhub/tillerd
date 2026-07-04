import type { SettingView } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { Store, useSelector } from "@tanstack/react-store";
import { getQueryClient, query, runCommand } from "@tillerd/client-bindings";
import React from "react";

import { broadcastInvalidate, onRemoteInvalidate } from "../crossWindowSync";
import {
  DEFAULT_THEME,
  GENERAL_STARTUP_WORKSPACE_KEY,
  THEME_KEY,
  VIEW_ACTIVE_WORKSPACE_KEY,
  isTheme,
  type Theme,
} from "./keys";
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

// Per-key latest-wins write queue. Two rapid writes to one key must not race on
// the wire (the older could commit last and win server-side), so at most one
// settingSet per key is in flight; a newer value replaces any queued one, and
// the query cache converges only when the chain drains. The settings-query
// observer must not overwrite a key with a concurrently-fetched snapshot while
// its chain is live.
const inFlightWrites = new Set<string>();
const queuedWrites = new Map<string, unknown>();

function writePending(key: string): boolean {
  return inFlightWrites.has(key) || queuedWrites.has(key);
}

// Test-only: reset module-level state between tests.
export function _resetForTests(): void {
  pendingWrites = [];
  hydrated = false;
  inFlightWrites.clear();
  queuedWrites.clear();
}

// Fire-and-forget: the interaction that caused the write never blocks on it. A
// failure that reaches the orchestrator is recorded there (`command-error`) and
// pushed back over the notification channel -- the renderer never records. On
// success sibling windows converge via the invalidation broadcast (the local
// store is already updated synchronously by setGlobalSetting).
function persist(key: string, value: unknown): void {
  if (inFlightWrites.has(key)) {
    queuedWrites.set(key, value);
    return;
  }
  send(key, value);
}

// Bumped whenever a key's chain settles; refreshFromSource re-fetches when a
// local write landed while its snapshot was in flight.
let settleStamp = 0;

function send(key: string, value: unknown): void {
  inFlightWrites.add(key);
  const advance = () => {
    inFlightWrites.delete(key);
    settleStamp++;
    if (queuedWrites.has(key)) {
      const next = queuedWrites.get(key);
      queuedWrites.delete(key);
      // A queued newer value continues the chain even after a failed write --
      // the user's latest choice must still reach disk.
      send(key, next);
      return false;
    }
    return true;
  };
  runCommand("settingSet", {
    scope: "global",
    projectId: null,
    key,
    valueJson: JSON.stringify(value),
  }).then(
    () => {
      if (!advance()) return;
      // Chain drained: converge the local query cache (the broadcast skips its
      // own window).
      void getQueryClient().invalidateQueries({ queryKey: ["settings"] });
      broadcastInvalidate([["settings"]]);
    },
    () => {
      advance();
    },
  );
}

function defaultResolve(): Promise<SettingView[]> {
  const opts = query("settingList", { scope: "global", projectId: null });
  // Hydration must reflect disk, not the restored (persisted) query cache -- that
  // snapshot can predate the previous run's last write. A zero stale floor makes
  // fetchQuery always hit the orchestrator.
  return getQueryClient().fetchQuery({ ...opts, staleTime: 0 });
}

// Synchronous pre-seed from the restored (persisted) query cache: the shell must
// render the last-known pointers immediately -- an interaction fired before the
// fresh disk read lands (fast click after launch) must scope like the visible
// UI, not like an empty store. Fills only missing keys so pre-hydration writes
// are never clobbered; the fresh read that follows corrects any staleness.
function seedFromCache(): void {
  const opts = query("settingList", { scope: "global", projectId: null });
  const cached = getQueryClient().getQueryData<SettingView[]>(opts.queryKey);
  if (!cached) return;
  settingsStore.setState((s) => {
    const values = { ...s.values };
    for (const e of cached) {
      if (!(e.key in values)) values[e.key] = e.value;
    }
    return { ...s, values };
  });
}

export async function hydrateSettings(
  resolve: () => Promise<SettingView[]> = defaultResolve,
): Promise<void> {
  seedFromCache();
  const entries = await resolve();
  const pending = pendingWrites;
  pendingWrites = [];
  hydrated = true;
  for (const w of pending) persist(w.key, w.value);
  const values: Record<string, unknown> = {};
  for (const e of entries) values[e.key] = e.value;
  // Pre-hydration changes win over the listed snapshot so they are not reverted.
  for (const w of pending) values[w.key] = w.value;
  // Startup workspace (General settings): a pinned workspace overrides the restored
  // last-active pointer exactly once, here at launch -- not in WorkspaceSwitcher's
  // per-render scopedId, which would fight the user's later in-session switches and
  // turn "startup default" into a perpetually-forced workspace.
  const startupWorkspace = values[GENERAL_STARTUP_WORKSPACE_KEY];
  if (typeof startupWorkspace === "string" && startupWorkspace) {
    values[VIEW_ACTIVE_WORKSPACE_KEY] = startupWorkspace;
  }
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

// Keep the store converged across windows: when a SIBLING window's write lands
// (remote ["settings"] invalidation), re-read the freshly invalidated settings
// query and re-seed the store. This window's own writes never route through
// here, so a local optimistic value cannot be reverted by its own feedback.
export function watchSettings(): () => void {
  return onRemoteInvalidate((keys) => {
    const touchesSettings = keys.some((key) => Array.isArray(key) && key[0] === "settings");
    if (touchesSettings && hydrated) void refreshFromSource();
  });
}

async function refreshFromSource(): Promise<void> {
  // Re-fetch while local writes settle during the snapshot: a chain that drained
  // mid-flight (writePending already false) would otherwise be reverted by data
  // fetched before that write committed. Bounded: user-paced writes drain fast.
  let entries: SettingView[];
  let attempts = 0;
  do {
    const stampBefore = settleStamp;
    entries = await defaultResolve();
    if (settleStamp === stampBefore) break;
  } while (++attempts < 3);

  const values: Record<string, unknown> = {};
  for (const e of entries) values[e.key] = e.value;
  settingsStore.setState((s) => {
    // A local write still in flight or queued wins over this snapshot, which
    // may have been fetched before the write committed.
    for (const key of Object.keys(s.values)) {
      if (writePending(key)) values[key] = s.values[key];
    }
    return { ...s, values };
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

export function useBoolGlobalSetting(
  key: string,
  fallback: boolean,
): { value: boolean; setValue: (value: boolean) => void } {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  const value = typeof raw === "boolean" ? raw : fallback;
  const setValue = React.useCallback((next: boolean) => setGlobalSetting(key, next), [key]);
  return { value, setValue };
}

export function useNumberGlobalSetting(
  key: string,
  fallback: number,
): { value: number; setValue: (value: number) => void } {
  const raw = useSelector(settingsStore, (s) => s.values[key]);
  const value = typeof raw === "number" && Number.isFinite(raw) ? raw : fallback;
  const setValue = React.useCallback((next: number) => setGlobalSetting(key, next), [key]);
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
