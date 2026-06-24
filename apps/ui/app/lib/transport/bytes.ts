// JSON number[] arrives only over invoke/Channel; binary WebSocket never delivers it, but accepting it here keeps one coercion site.
export function toBytes(data: unknown): Uint8Array | null {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (Array.isArray(data)) return new Uint8Array(data as number[]);
  return null;
}
