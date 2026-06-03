// Client → Daemon (partial — capabilities in hello)
export type ClientCapability = "snapshot";

// Daemon → Client
export type HelloAckFrame = { type: "hello-ack"; version: number; daemonVersion: string; capabilities?: ClientCapability[] };
export type SpawnAckFrame = { type: "spawn-ack"; sessionId: string; pid: number };
export type ListAckFrame = { type: "list-ack"; ids: string[] };
export type DataFrameMeta = { type: "data"; sessionId: string; bodyLen: number };
export type ExitFrame = {
  type: "exit";
  sessionId: string;
  qualifier: string;
  raw?: { code?: number | null; signal?: string | null; signalName?: string; signalMeaning?: string; signalCategory?: string };
};
export type HookFrame = { type: "hook"; sessionId: string; payload: unknown };
export type ErrorFrame = { type: "error"; code: string; message: string; sessionId?: string };

export interface SnapshotCell { char: string; fg: number; bg: number; attrs: number }
export type SnapshotFrame = {
  type: "snapshot";
  sessionId: string;
  rows: number;
  cols: number;
  cells: SnapshotCell[][];
  cursor: { x: number; y: number };
};

export type DaemonFrame =
  | HelloAckFrame
  | SpawnAckFrame
  | ListAckFrame
  | DataFrameMeta
  | ExitFrame
  | HookFrame
  | ErrorFrame
  | SnapshotFrame;

export const SUPPORTED_VERSIONS = [1] as const;
export const CURRENT_VERSION = 1;

export function parseDaemonFrame(meta: unknown): DaemonFrame | null {
  const m = meta as Record<string, unknown>;
  if (m && typeof m["type"] === "string") return m as unknown as DaemonFrame;
  return null;
}
