import type { SessionStatus, ContentEvent, ExitEvent } from "./events";
import type { AtError } from "./errors";
import type { AgentDefinition } from "./adapter";

export interface SessionOptions {
  cwd?: string;
  cols?: number;
  rows?: number;
  resume?: string;
  startupTimeoutMs?: number;
  shutdownGraceMs?: number;
  idleTimeoutMs?: number;
  sendQueueCapacity?: number;
  captureRawIo?: boolean;
  /** Pre-minted session identifier. Overrides the internally generated UUID. */
  sessionId?: string;
  /** Gate HTTP base URL. When present, ATHING_GATE_URL is injected into the daemon spawn env. */
  gateUrl?: string;
  /** Pre-minted bearer token registered with the gate admin. Overrides the internally generated token. */
  gateToken?: string;
}

export interface AgentSession {
  readonly sessionId: string;
  send(text: string): void;
  input(bytes: Uint8Array): void;
  interrupt(): void;
  resize(cols: number, rows: number): void;
  kill(): Promise<ExitEvent>;
  stop(): Promise<ExitEvent>;
  onData(handler: (bytes: Uint8Array) => void): () => void;
  onStatus(handler: (status: SessionStatus) => void): () => void;
  /**
   * Terminal-plane status (`IDLE` | `WORKING`) derived from the OS view of the
   * session process — distinct from {@link onStatus}, the agent's hook-derived
   * lifecycle. Co-equal signals; combine them only as a presentation choice.
   */
  onTerminalStatus(handler: (status: SessionStatus) => void): () => void;
  onContent(handler: (event: ContentEvent) => void): () => void;
  onError(handler: (error: AtError) => void): () => void;
  onExit(handler: (event: ExitEvent) => void): () => void;
}

export interface Engine {
  start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession>;
  reconnect(
    sessionId: string,
    adapter: AgentDefinition,
    options?: SessionOptions,
  ): Promise<AgentSession>;
  listSessions(): Promise<string[]>;
  shutdown(): Promise<void>;
}
