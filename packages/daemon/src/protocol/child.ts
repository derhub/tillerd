import * as v from "valibot";

// ── Daemon → Child ───────────────────────────────────────────────────────────

export const ChildSpawnSchema = v.object({
  type: v.literal("spawn"),
  command: v.string(),
  args: v.array(v.string()),
  flags: v.array(v.string()),
  cwd: v.string(),
  cols: v.number(),
  rows: v.number(),
  env: v.record(v.string(), v.string()),
});

export const ChildInputSchema = v.object({
  type: v.literal("input"),
});

export const ChildResizeSchema = v.object({
  type: v.literal("resize"),
  cols: v.number(),
  rows: v.number(),
});

export const ChildInterruptSchema = v.object({
  type: v.literal("interrupt"),
});

export const ChildGetFdSchema = v.object({
  type: v.literal("get-fd"),
});

export const ChildAdoptSchema = v.object({
  type: v.literal("adopt"),
  fdIndex: v.number(),
  pid: v.number(),
  cols: v.number(),
  rows: v.number(),
});

export const DaemonToChildSchema = v.union([
  ChildSpawnSchema,
  ChildInputSchema,
  ChildResizeSchema,
  ChildInterruptSchema,
  ChildGetFdSchema,
  ChildAdoptSchema,
]);

export type DaemonToChild = v.InferOutput<typeof DaemonToChildSchema>;
export type ChildSpawnMsg = v.InferOutput<typeof ChildSpawnSchema>;
export type ChildAdoptMsg = v.InferOutput<typeof ChildAdoptSchema>;

// ── Child → Daemon ───────────────────────────────────────────────────────────

export const ChildSpawnAckSchema = v.object({
  type: v.literal("spawn-ack"),
  pid: v.number(),
});

export const ChildDataSchema = v.object({
  type: v.literal("data"),
  bodyLen: v.number(),
});

export const ChildExitSchema = v.object({
  type: v.literal("exit"),
  code: v.nullable(v.number()),
  signal: v.nullable(v.string()),
});

export const ChildFdSchema = v.object({
  type: v.literal("fd"),
  fd: v.number(),
});

export const ChildToDaemonSchema = v.union([
  ChildSpawnAckSchema,
  ChildDataSchema,
  ChildExitSchema,
  ChildFdSchema,
]);

export type ChildToDaemon = v.InferOutput<typeof ChildToDaemonSchema>;
export type ChildSpawnAck = v.InferOutput<typeof ChildSpawnAckSchema>;
export type ChildDataMeta = v.InferOutput<typeof ChildDataSchema>;
export type ChildExit = v.InferOutput<typeof ChildExitSchema>;
export type ChildFdMsg = v.InferOutput<typeof ChildFdSchema>;

export function parseDaemonToChild(meta: unknown): DaemonToChild | null {
  try {
    return v.parse(DaemonToChildSchema, meta);
  } catch {
    return null;
  }
}

export function parseChildToDaemon(meta: unknown): ChildToDaemon | null {
  try {
    return v.parse(ChildToDaemonSchema, meta);
  } catch {
    return null;
  }
}
