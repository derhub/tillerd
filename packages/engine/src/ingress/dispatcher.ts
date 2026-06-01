import type { HookEvent, AgentDefinition } from "@athing/sdk";
import type { Logger } from "../logger";

export type HookHandler = (event: HookEvent) => void;

export interface SessionEntry {
  token: string;
  adapter: AgentDefinition;
  handler: HookHandler;
}

export class HookDispatcher {
  private sessions = new Map<string, SessionEntry>();
  private processed = new Set<string>();

  constructor(private readonly logger: Logger) {}

  register(sessionId: string, token: string, adapter: AgentDefinition, handler: HookHandler): void {
    this.sessions.set(sessionId, { token, adapter, handler });
  }

  unregister(sessionId: string): void {
    this.sessions.delete(sessionId);
  }

  dispatch(sessionId: string, token: string, rawPayload: unknown): boolean {
    const entry = this.sessions.get(sessionId);
    if (!entry) {
      this.logger.debug("hook.dispatch: unknown session", { sessionId });
      return false;
    }

    if (token !== entry.token) {
      this.logger.warn("hook.dispatch: token mismatch", { sessionId });
      return false;
    }

    let event: HookEvent;
    try {
      event = entry.adapter.parseHook(rawPayload);
    } catch (err) {
      this.logger.warn("hook.dispatch: parseHook failed", { sessionId, err: String(err) });
      return false;
    }

    const idempotencyKey = `${sessionId}:${event.type}:${JSON.stringify(event.payload ?? null)}`;
    if (this.processed.has(idempotencyKey)) {
      this.logger.debug("hook.dispatch: duplicate, skipping", { sessionId, type: event.type });
      return true;
    }
    this.processed.add(idempotencyKey);
    this.pruneProcessed();

    entry.handler(event);
    return true;
  }

  dispatchDirect(event: HookEvent): void {
    const entry = this.sessions.get(event.sessionId);
    if (!entry) {
      this.logger.debug("hook.dispatchDirect: unknown session", { sessionId: event.sessionId });
      return;
    }

    const idempotencyKey = `${event.sessionId}:${event.type}:${JSON.stringify(event.payload ?? null)}`;
    if (this.processed.has(idempotencyKey)) {
      this.logger.debug("hook.dispatchDirect: duplicate, skipping", {
        sessionId: event.sessionId,
        type: event.type,
      });
      return;
    }
    this.processed.add(idempotencyKey);
    this.pruneProcessed();

    entry.handler(event);
  }

  private pruneProcessed(): void {
    if (this.processed.size > 10_000) {
      const iter = this.processed.values();
      for (let i = 0; i < 1_000; i++) {
        const val = iter.next().value;
        if (val) this.processed.delete(val);
      }
    }
  }
}
