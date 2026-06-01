import type { HookEvent, SessionStatus, AgentDefinition } from "@athing/sdk";

export class Engine {
  private status: SessionStatus = "IDLE";
  private adapter: AgentDefinition | null = null;

  constructor(adapter: AgentDefinition) {
    this.adapter = adapter;
  }

  parseHook(_event: HookEvent): SessionStatus {
    return this.status;
  }
}
