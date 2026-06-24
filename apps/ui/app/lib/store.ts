import { Store, useSelector } from "@tanstack/react-store";

// Client UI state only. Server data (projects/sessions/workspaces) lives in the Query cache; never duplicated here.
interface UiState {
  activeWorkspaceId: string | null;
}

export const uiStore = new Store<UiState>({ activeWorkspaceId: null });

export function setActiveWorkspace(id: string | null): void {
  uiStore.setState((s) => ({ ...s, activeWorkspaceId: id }));
}

export function useActiveWorkspace(): string | null {
  return useSelector(uiStore, (s) => s.activeWorkspaceId);
}
