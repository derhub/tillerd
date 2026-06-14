import type { OrchestratorStatus } from "./status";
import { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD } from "./status";
import type { ServiceHealth } from "./service-health";
import { SERVICE_HEALTH_METHOD } from "./service-health";
import {
  createWorkspaceClient,
  type WorkspaceClient,
  type Project,
  type Session,
  type CreateProjectArgs,
  type RenameProjectArgs,
  type ArchiveProjectArgs,
  type DeleteProjectArgs,
  type ReorderProjectArgs,
  type CreateSessionArgs,
  type RenameSessionArgs,
  type ListSessionsArgs,
  type ArchiveSessionArgs,
  type DeleteSessionArgs,
  type ReorderSessionArgs,
  type SetSessionLayoutArgs,
  type GetSessionLayoutArgs,
} from "./workspace";
import {
  createSettingsClient,
  type SettingsClient,
  type SettingScope,
  type SettingEntry,
  type GetSettingArgs,
  type SetSettingArgs,
  type ListSettingsArgs,
} from "./settings";

export interface OrchestratorHostTransport {
  invoke<T>(method: string, args?: Record<string, unknown>): Promise<T>;
  listen(event: string, handler: (payload: OrchestratorStatus) => void): Promise<() => void>;
}

export interface OrchestratorClient extends WorkspaceClient, SettingsClient {
  status(): Promise<OrchestratorStatus>;
  subscribe(handler: (status: OrchestratorStatus) => void): Promise<() => void>;
  /** Read-only per-service health snapshot (gate, daemon). Re-query on a status event. */
  serviceHealth(): Promise<ServiceHealth[]>;
}

export function createOrchestratorClient(transport: OrchestratorHostTransport): OrchestratorClient {
  const workspace = createWorkspaceClient(transport);
  const settings = createSettingsClient(transport);
  return {
    status: () => transport.invoke<OrchestratorStatus>(ORCHESTRATOR_STATUS_METHOD),
    subscribe: (handler) => transport.listen(ORCHESTRATOR_STATUS_EVENT, handler),
    serviceHealth: () => transport.invoke<ServiceHealth[]>(SERVICE_HEALTH_METHOD),
    ...workspace,
    ...settings,
  };
}

export type {
  Project,
  Session,
  CreateProjectArgs,
  RenameProjectArgs,
  ArchiveProjectArgs,
  DeleteProjectArgs,
  ReorderProjectArgs,
  CreateSessionArgs,
  RenameSessionArgs,
  ListSessionsArgs,
  ArchiveSessionArgs,
  DeleteSessionArgs,
  ReorderSessionArgs,
  SetSessionLayoutArgs,
  GetSessionLayoutArgs,
  SettingScope,
  SettingEntry,
  GetSettingArgs,
  SetSettingArgs,
  ListSettingsArgs,
};
