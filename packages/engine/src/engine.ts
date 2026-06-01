import type { Engine as IEngine, AgentSession, AgentDefinition, SessionOptions } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import { HookDispatcher } from "./ingress/dispatcher";
import { HookReceiver } from "./ingress/receiver";
import { installHooks } from "./ingress/install";
import { AgentSessionImpl, fillOptions } from "./session/session";
import { createLogger } from "./logger";
import { checkCliVersion } from "./pty/resolve";
import { randomUUID } from "node:crypto";

class EngineImpl implements IEngine {
  private sessions = new Map<string, AgentSessionImpl>();
  private receiver: HookReceiver;
  private dispatcher: HookDispatcher;
  private logger = createLogger();
  private installedAdapters = new Set<string>();
  private verifiedVersions = new Set<string>();
  private shutdown_ = false;

  constructor() {
    this.dispatcher = new HookDispatcher(this.logger);
    this.receiver = new HookReceiver(this.dispatcher, this.logger);
    this.receiver.start();
  }

  async start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession> {
    if (this.shutdown_) throw new AtError("TransportClosed", "Engine is shut down");

    if (!this.verifiedVersions.has(adapter.name)) {
      checkCliVersion(adapter.launch.command, adapter.cliVersionRange);
      this.verifiedVersions.add(adapter.name);
    }

    if (!this.installedAdapters.has(adapter.name)) {
      installHooks(adapter.hookInstall, this.logger);
      this.installedAdapters.add(adapter.name);
    }

    const sessionId = randomUUID();
    const opts = fillOptions(options);
    const session = new AgentSessionImpl(
      sessionId,
      adapter,
      opts,
      this.dispatcher,
      this.receiver.url,
    );

    this.sessions.set(sessionId, session);
    session.onExit(() => this.sessions.delete(sessionId));
    // setTimeout(0) defers past the caller's await continuation so handlers are registered first.
    setTimeout(() => session.start(), 0);

    return session;
  }

  async shutdown(): Promise<void> {
    if (this.shutdown_) return;
    this.shutdown_ = true;

    const kills = [...this.sessions.values()].map((s) => s.kill());
    await Promise.allSettled(kills);
    this.sessions.clear();
    this.receiver.stop();
  }

  dispatchHook(event: Parameters<HookDispatcher["dispatchDirect"]>[0]): void {
    this.dispatcher.dispatchDirect(event);
  }
}

export function createEngine(): IEngine & { dispatchHook: EngineImpl["dispatchHook"] } {
  return new EngineImpl();
}
