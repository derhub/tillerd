import type {
  Engine as IEngine,
  AgentSession,
  AgentDefinition,
  SessionOptions,
  DaemonTransport,
  HookSource,
  Logger,
} from "@athing/sdk";
import { AtError } from "@athing/sdk";
import { AgentSessionProxy, fillProxyOptions } from "./daemon/proxy";

export interface EngineDeps {
  transport: DaemonTransport;
  logger: Logger;
  /** Runtime directory ($ATHING_DIR) injected into the agent so its hook client derives the gate socket. */
  athingDir: string;
  resolvedCommand: string;
  hookSource?: HookSource;
}

class EngineImpl implements IEngine {
  private proxies = new Map<string, AgentSessionProxy>();
  private shutdown_ = false;

  constructor(private readonly deps: EngineDeps) {}

  async start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession> {
    if (this.shutdown_) throw new AtError("TransportClosed", "Engine is shut down");

    const sessionId = options?.sessionId ?? options?.resume ?? crypto.randomUUID();
    const opts = fillProxyOptions(options);
    const proxy = new AgentSessionProxy(
      sessionId,
      adapter,
      opts,
      this.deps.transport,
      "spawn",
      this.deps.athingDir,
      this.deps.logger,
      this.deps.resolvedCommand,
      this.deps.hookSource,
    );
    this.proxies.set(sessionId, proxy);
    proxy.onExit(() => this.proxies.delete(sessionId));
    setTimeout(() => proxy.start(), 0);
    return proxy;
  }

  async reconnect(
    sessionId: string,
    adapter: AgentDefinition,
    options?: SessionOptions,
  ): Promise<AgentSession> {
    if (this.shutdown_) throw new AtError("TransportClosed", "Engine is shut down");

    const knownIds = await this.deps.transport.list();
    if (!knownIds.includes(sessionId)) {
      throw new AtError("TransportClosed", `Session ${sessionId} not found in daemon`);
    }

    const opts = fillProxyOptions(options);
    const proxy = new AgentSessionProxy(
      sessionId,
      adapter,
      opts,
      this.deps.transport,
      "subscribe",
      this.deps.athingDir,
      this.deps.logger,
      this.deps.resolvedCommand,
      this.deps.hookSource,
    );
    this.proxies.set(sessionId, proxy);
    proxy.onExit(() => this.proxies.delete(sessionId));
    setTimeout(() => proxy.start(), 0);
    return proxy;
  }

  async listSessions(): Promise<string[]> {
    return this.deps.transport.list();
  }

  async shutdown(): Promise<void> {
    if (this.shutdown_) return;
    this.shutdown_ = true;

    try {
      for (const sessionId of this.proxies.keys()) {
        this.deps.transport.send({ op: "unsubscribe", sessionId });
      }
      this.deps.transport.disconnect();
    } catch {
      // ignore
    }
    this.proxies.clear();
  }
}

export function createEngine(deps: EngineDeps): IEngine {
  return new EngineImpl(deps);
}
