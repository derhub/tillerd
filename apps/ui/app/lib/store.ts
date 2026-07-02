import { Store, useSelector } from "@tanstack/react-store";
import React from "react";

// Client UI state only. Server data (projects/sessions/workspaces) lives in the Query cache; never duplicated here.
interface UiState {
  activeWorkspaceId: string | null;
  activeProjectId: string | null;
  commandCenterOpen: boolean;
  expandedProjectIds: Record<string, boolean>;
}

const LOCAL_STORAGE_KEY = "tillerd:ui-state";

function getInitialState(): UiState {
  const defaults: UiState = {
    activeWorkspaceId: null,
    activeProjectId: null,
    commandCenterOpen: false,
    expandedProjectIds: {},
  };

  if (typeof window === "undefined" || typeof window.localStorage === "undefined") {
    return defaults;
  }

  try {
    const raw = window.localStorage.getItem(LOCAL_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === "object") {
        return {
          ...defaults,
          activeWorkspaceId:
            typeof parsed.activeWorkspaceId === "string" ? parsed.activeWorkspaceId : null,
          activeProjectId:
            typeof parsed.activeProjectId === "string" ? parsed.activeProjectId : null,
          expandedProjectIds:
            parsed.expandedProjectIds && typeof parsed.expandedProjectIds === "object"
              ? parsed.expandedProjectIds
              : {},
        };
      }
    }
  } catch (e) {
    console.error("Failed to load uiStore state from localStorage", e);
  }
  return defaults;
}

const initialState = getInitialState();

export const uiStore = new Store<UiState>(initialState);

if (typeof window !== "undefined" && typeof window.localStorage !== "undefined") {
  uiStore.subscribe(() => {
    try {
      const state = uiStore.state;
      window.localStorage.setItem(
        LOCAL_STORAGE_KEY,
        JSON.stringify({
          activeWorkspaceId: state.activeWorkspaceId,
          activeProjectId: state.activeProjectId,
          expandedProjectIds: state.expandedProjectIds,
        }),
      );
    } catch (e) {
      console.error("Failed to save uiStore state to localStorage", e);
    }
  });
}

export function setActiveWorkspace(id: string | null): void {
  uiStore.setState((s) => ({ ...s, activeWorkspaceId: id }));
}

export function useActiveWorkspace(): string | null {
  return useSelector(uiStore, (s) => s.activeWorkspaceId);
}

export function setActiveProject(id: string | null): void {
  uiStore.setState((s) => ({
    ...s,
    activeProjectId: id,
    expandedProjectIds: id ? { ...s.expandedProjectIds, [id]: true } : s.expandedProjectIds,
  }));
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
  uiStore.setState((s) => ({
    ...s,
    expandedProjectIds: { ...s.expandedProjectIds, [projectId]: expanded },
  }));
}

export function useProjectExpanded(projectId: string) {
  const expanded = useSelector(uiStore, (s) => !!s.expandedProjectIds[projectId]);
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
    activeWorkspaceId: null,
    activeProjectId: null,
    commandCenterOpen: false,
    expandedProjectIds: {},
  }));
}
