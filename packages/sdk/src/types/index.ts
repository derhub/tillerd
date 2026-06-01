export type SessionStatus = "IDLE" | "WORKING" | "WAITING_INPUT" | "DONE";

export interface HookEvent {
  type: string;
  timestamp: number;
  data: unknown;
}

export interface AgentDefinition {
  name: string;
  description: string;
}
