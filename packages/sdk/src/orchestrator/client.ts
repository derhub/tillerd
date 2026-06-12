import type { OrchestratorStatus } from "./status";
import { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD } from "./status";
import {
  createWorkspaceClient,
  type WorkspaceClient,
  type Project,
  type Session,
  type CreateProjectArgs,
  type RenameProjectArgs,
  type ArchiveProjectArgs,
  type CreateSessionArgs,
  type RenameSessionArgs,
  type ListSessionsArgs,
  type ArchiveSessionArgs,
  type SetSessionLayoutArgs,
  type GetSessionLayoutArgs,
} from "./workspace";

export interface OrchestratorHostTransport {
  invoke<T>(method: string, args?: Record<string, unknown>): Promise<T>;
  listen(event: string, handler: (payload: OrchestratorStatus) => void): Promise<() => void>;
}

export interface OrchestratorClient extends WorkspaceClient {
  status(): Promise<OrchestratorStatus>;
  subscribe(handler: (status: OrchestratorStatus) => void): Promise<() => void>;
}

export function createOrchestratorClient(transport: OrchestratorHostTransport): OrchestratorClient {
  const workspace = createWorkspaceClient(transport);
  return {
    status: () => transport.invoke<OrchestratorStatus>(ORCHESTRATOR_STATUS_METHOD),
    subscribe: (handler) => transport.listen(ORCHESTRATOR_STATUS_EVENT, handler),
    ...workspace,
  };
}

export type {
  Project,
  Session,
  CreateProjectArgs,
  RenameProjectArgs,
  ArchiveProjectArgs,
  CreateSessionArgs,
  RenameSessionArgs,
  ListSessionsArgs,
  ArchiveSessionArgs,
  SetSessionLayoutArgs,
  GetSessionLayoutArgs,
};
