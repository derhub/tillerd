import type { AtError } from "./errors";
import type { SessionStatus, ContentEvent, ExitEvent } from "./events";

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
   * session process -- distinct from {@link onStatus}, the agent's hook-derived
   * lifecycle. Co-equal signals; combine them only as a presentation choice.
   */
  onTerminalStatus(handler: (status: SessionStatus) => void): () => void;
  onContent(handler: (event: ContentEvent) => void): () => void;
  onError(handler: (error: AtError) => void): () => void;
  onExit(handler: (event: ExitEvent) => void): () => void;
}
