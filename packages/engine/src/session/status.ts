import type { HookEvent, HookEventType, SessionStatus } from "@tillerd/sdk";

const STATUS_MAP: Record<HookEventType, SessionStatus> = {
  SessionStart: "IDLE",
  UserPromptSubmit: "WORKING",
  PostToolUse: "WORKING",
  PermissionRequest: "WAITING_INPUT",
  Stop: "IDLE",
  SessionEnd: "DONE",
};

type StatusHandler = (status: SessionStatus) => void;

export class StatusMapper {
  private current: SessionStatus = "IDLE";
  private handlers = new Set<StatusHandler>();

  apply(event: HookEvent): void {
    const next = STATUS_MAP[event.type];
    if (next === undefined || next === this.current) return;
    this.current = next;
    for (const h of this.handlers) h(next);
  }

  get(): SessionStatus {
    return this.current;
  }

  onChange(handler: StatusHandler): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }
}
