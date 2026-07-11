import { Store, useSelector } from "@tanstack/react-store";
import React from "react";

import { setGlobalSetting, settingsStore } from "~/lib/settings/context";
import {
  sidebarExpandedKey,
  VIEW_ACTIVE_WORKSPACE_KEY,
  WORKBENCH_PREFIX,
} from "~/lib/settings/keys";

// Window-scoped, ephemeral UI state only. The durable view pointers (active
// workspace, sidebar expansion, last session -- ) are settings-store keys
// read through the settings bootstrap; server data lives in the Query cache.
// activeProjectId stays per-window: it is derived from the window's URL intent,
// not a cross-window position.
interface UiState {
  activeProjectId: string | null;
  commandCenterOpen: boolean;
}

export const uiStore = new Store<UiState>({
  activeProjectId: null,
  commandCenterOpen: false,
});

export function setActiveWorkspace(id: string | null): void {
  setGlobalSetting(VIEW_ACTIVE_WORKSPACE_KEY, id);
}

export function useActiveWorkspace(): string | null {
  const raw = useSelector(settingsStore, (s) => s.values[VIEW_ACTIVE_WORKSPACE_KEY]);
  return typeof raw === "string" && raw ? raw : null;
}

export function setActiveProject(id: string | null): void {
  uiStore.setState((s) => ({ ...s, activeProjectId: id }));
  if (id) setProjectExpanded(id, true);
}

export function useActiveProject(): string | null {
  return useSelector(uiStore, (s) => s.activeProjectId);
}

export function setCommandCenterOpen(open: boolean): void {
  uiStore.setState((s) => ({ ...s, commandCenterOpen: open }));
}

export function useCommandCenterOpen() {
  const open = useSelector(uiStore, (s) => s.commandCenterOpen);
  const setOpen = React.useCallback((val: boolean) => {
    setCommandCenterOpen(val);
  }, []);
  return [open, setOpen] as const;
}

export function setProjectExpanded(projectId: string, expanded: boolean): void {
  setGlobalSetting(sidebarExpandedKey(projectId), expanded);
}

// Absent pointer means expanded (spec: default expanded); only an explicit stored
// `false` collapses. Persisted collapse thus survives a restart, and a fresh
// project group shows its sessions without a first click.
export function useProjectExpanded(projectId: string) {
  const expanded = useSelector(
    settingsStore,
    (s) => s.values[sidebarExpandedKey(projectId)] !== false,
  );
  const setExpanded = React.useCallback(
    (val: boolean) => {
      setProjectExpanded(projectId, val);
    },
    [projectId],
  );
  return [expanded, setExpanded] as const;
}

export function resetUiStore(): void {
  uiStore.setState(() => ({
    activeProjectId: null,
    commandCenterOpen: false,
  }));
  // Strip the view-pointer keys without persisting: test/reset hygiene, not a write.
  settingsStore.setState((s) => {
    const values = { ...s.values };
    for (const key of Object.keys(values)) {
      if (
        key === VIEW_ACTIVE_WORKSPACE_KEY ||
        key.startsWith("sidebar.expanded.") ||
        key.startsWith(WORKBENCH_PREFIX)
      ) {
        delete values[key];
      }
    }
    return { ...s, values };
  });
}
