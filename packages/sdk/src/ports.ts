import type { DaemonFrame } from "./protocol/messages";
import type { HookEvent } from "./types/events";

export type FrameHandler = (frame: DaemonFrame, body: Uint8Array | null) => void;

export interface DaemonTransport {
  connect(): Promise<void>;
  send(meta: object, body?: Uint8Array): void;
  subscribe(sessionId: string, handler: FrameHandler): () => void;
  list(): Promise<string[]>;
  onClose(handler: () => void): () => void;
  disconnect(): void;
}

export interface FileSource {
  size(path: string): Promise<number | null>;
  read(path: string, offset: number, length: number): Promise<Uint8Array>;
}

export interface HookSource {
  subscribe(sessionId: string): AsyncIterableIterator<HookEvent>;
}
