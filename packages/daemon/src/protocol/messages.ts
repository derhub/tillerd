import * as v from "valibot";

// ── Client → Daemon ──────────────────────────────────────────────────────────

export const HelloSchema = v.object({
  type: v.literal("hello"),
  versions: v.array(v.number()),
  capabilities: v.optional(v.array(v.string())),
});

export const SpawnSchema = v.object({
  type: v.literal("spawn"),
  sessionId: v.string(),
  resume: v.optional(v.string()),
  command: v.string(),
  args: v.array(v.string()),
  flags: v.array(v.string()),
  hookSocketPath: v.string(),
  token: v.string(),
  cols: v.number(),
  rows: v.number(),
  cwd: v.string(),
});

export const KillSchema = v.object({
  type: v.literal("kill"),
  sessionId: v.string(),
});

export const StopSchema = v.object({
  type: v.literal("stop"),
  sessionId: v.string(),
});

export const ListSchema = v.object({
  type: v.literal("list"),
});

export const SubscribeSchema = v.object({
  type: v.literal("subscribe"),
  sessionId: v.string(),
});

export const UnsubscribeSchema = v.object({
  type: v.literal("unsubscribe"),
  sessionId: v.string(),
});

export const InputSchema = v.object({
  type: v.literal("input"),
  sessionId: v.string(),
});

export const ResizeSchema = v.object({
  type: v.literal("resize"),
  sessionId: v.string(),
  cols: v.number(),
  rows: v.number(),
});

export const InterruptSchema = v.object({
  type: v.literal("interrupt"),
  sessionId: v.string(),
});

export const AckSchema = v.object({
  type: v.literal("ack"),
  sessionId: v.string(),
  bytes: v.number(),
});

export const UpgradeSchema = v.object({
  type: v.literal("upgrade"),
});

export const ClientFrameSchema = v.union([
  HelloSchema,
  SpawnSchema,
  KillSchema,
  StopSchema,
  ListSchema,
  SubscribeSchema,
  UnsubscribeSchema,
  InputSchema,
  ResizeSchema,
  InterruptSchema,
  AckSchema,
  UpgradeSchema,
]);

export type ClientFrame = v.InferOutput<typeof ClientFrameSchema>;
export type HelloFrame = v.InferOutput<typeof HelloSchema>;
export type SpawnFrame = v.InferOutput<typeof SpawnSchema>;
export type StopFrame = v.InferOutput<typeof StopSchema>;
export type InputFrame = v.InferOutput<typeof InputSchema>;
export type AckFrame = v.InferOutput<typeof AckSchema>;

// ── Daemon → Client ──────────────────────────────────────────────────────────

export const SpawnAckSchema = v.object({
  type: v.literal("spawn-ack"),
  sessionId: v.string(),
  pid: v.number(),
});

export const ListAckSchema = v.object({
  type: v.literal("list-ack"),
  ids: v.array(v.string()),
});

export const DataFrameSchema = v.object({
  type: v.literal("data"),
  sessionId: v.string(),
  bodyLen: v.number(),
});

const ExitRawSchema = v.optional(v.object({
  code: v.optional(v.nullable(v.number())),
  signal: v.optional(v.nullable(v.string())),
  signalName: v.optional(v.string()),
  signalMeaning: v.optional(v.string()),
  signalCategory: v.optional(v.string()),
}));

export const ExitFrameSchema = v.object({
  type: v.literal("exit"),
  sessionId: v.string(),
  qualifier: v.string(),
  raw: ExitRawSchema,
});

export const HookFrameSchema = v.object({
  type: v.literal("hook"),
  sessionId: v.string(),
  payload: v.unknown(),
});

export const ErrorFrameSchema = v.object({
  type: v.literal("error"),
  code: v.string(),
  message: v.string(),
  sessionId: v.optional(v.string()),
});

const SnapshotCellSchema = v.object({
  char: v.string(),
  fg: v.number(),
  bg: v.number(),
  attrs: v.number(),
});

export const SnapshotFrameSchema = v.object({
  type: v.literal("snapshot"),
  sessionId: v.string(),
  rows: v.number(),
  cols: v.number(),
  cells: v.array(v.array(SnapshotCellSchema)),
  cursor: v.object({ x: v.number(), y: v.number() }),
});

export const HelloAckSchema = v.object({
  type: v.literal("hello-ack"),
  version: v.number(),
  daemonVersion: v.string(),
  capabilities: v.optional(v.array(v.string())),
});

export const DaemonFrameSchema = v.union([
  HelloAckSchema,
  SpawnAckSchema,
  ListAckSchema,
  DataFrameSchema,
  ExitFrameSchema,
  HookFrameSchema,
  ErrorFrameSchema,
  SnapshotFrameSchema,
]);

export type DaemonFrame = v.InferOutput<typeof DaemonFrameSchema>;
export type HelloAckFrame = v.InferOutput<typeof HelloAckSchema>;
export type SpawnAckFrame = v.InferOutput<typeof SpawnAckSchema>;
export type DataFrameMeta = v.InferOutput<typeof DataFrameSchema>;
export type ExitFrame = v.InferOutput<typeof ExitFrameSchema>;
export type HookFrame = v.InferOutput<typeof HookFrameSchema>;
export type ErrorFrame = v.InferOutput<typeof ErrorFrameSchema>;
export type SnapshotFrame = v.InferOutput<typeof SnapshotFrameSchema>;

export const SUPPORTED_VERSIONS = [1] as const;
export const CURRENT_VERSION = 1;

export function parseClientFrame(meta: unknown): ClientFrame | null {
  try {
    return v.parse(ClientFrameSchema, meta);
  } catch {
    return null;
  }
}

export function parseDaemonFrame(meta: unknown): DaemonFrame | null {
  try {
    return v.parse(DaemonFrameSchema, meta);
  } catch {
    return null;
  }
}
