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
