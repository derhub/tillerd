import type { Engine as IEngine, AgentSession, AgentDefinition, SessionOptions } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import { prepareNotifyScript } from "./ingress/install";
import { createLogger } from "@athing/logger";
import { checkCliVersion } from "./pty/resolve";
import { randomUUID } from "node:crypto";
import { adoptOrSpawn } from "./daemon/supervisor";
import { AgentSessionProxy, fillProxyOptions } from "./daemon/proxy";
import type { DaemonClient } from "./daemon/client";

class EngineImpl implements IEngine {
  private proxies = new Map<string, AgentSessionProxy>();
  private logger = createLogger();
  private installedAdapters = new Set<string>();
  private verifiedVersions = new Set<string>();
  private shutdown_ = false;
  private daemonClientPromise: Promise<DaemonClient> | null = null;

  private getDaemonClient(): Promise<DaemonClient> {
    if (!this.daemonClientPromise) {
      this.daemonClientPromise = adoptOrSpawn();
    }
    return this.daemonClientPromise;
  }

  async start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession> {
    if (this.shutdown_) throw new AtError("TransportClosed", "Engine is shut down");

    if (!this.verifiedVersions.has(adapter.name)) {
      checkCliVersion(adapter.launch.command, adapter.cliVersionRange);
      this.verifiedVersions.add(adapter.name);
    }

    if (!this.installedAdapters.has(adapter.name)) {
      const { command, updated } = prepareNotifyScript();
      if (updated) this.logger.info("notify script updated");
      adapter.installHooks(command, this.logger);
      this.installedAdapters.add(adapter.name);
    }

    const client = await this.getDaemonClient();
    const sessionId = randomUUID();
    const opts = fillProxyOptions(options);
    const proxy = new AgentSessionProxy(sessionId, adapter, opts, client, "spawn");
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

    const client = await this.getDaemonClient();
    const knownIds = await client.list();
    if (!knownIds.includes(sessionId)) {
      throw new AtError("TransportClosed", `Session ${sessionId} not found in daemon`);
    }

    const opts = fillProxyOptions(options);
    const proxy = new AgentSessionProxy(sessionId, adapter, opts, client, "subscribe");
    this.proxies.set(sessionId, proxy);
    proxy.onExit(() => this.proxies.delete(sessionId));
    setTimeout(() => proxy.start(), 0);
    return proxy;
  }

  async listSessions(): Promise<string[]> {
    const client = await this.getDaemonClient();
    return client.list();
  }

  async shutdown(): Promise<void> {
    if (this.shutdown_) return;
    this.shutdown_ = true;

    if (this.daemonClientPromise) {
      try {
        const client = await this.daemonClientPromise;
        for (const sessionId of this.proxies.keys()) {
          client.send({ op: "unsubscribe", sessionId });
        }
        client.disconnect();
      } catch {
        // ignore
      }
    }
    this.proxies.clear();
  }
}

export function createEngine(): IEngine {
  return new EngineImpl();
}
