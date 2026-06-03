const HEADER_SIZE = 4;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

export function encodeFrame(meta: unknown, body?: Uint8Array): Uint8Array {
  const metaBytes = encoder.encode(JSON.stringify(meta));
  const payloadLen = body ? metaBytes.length + 1 + body.length : metaBytes.length;
  const out = new Uint8Array(HEADER_SIZE + payloadLen);

  out[0] = (payloadLen >>> 24) & 0xff;
  out[1] = (payloadLen >>> 16) & 0xff;
  out[2] = (payloadLen >>> 8) & 0xff;
  out[3] = payloadLen & 0xff;

  out.set(metaBytes, HEADER_SIZE);
  if (body) {
    out[HEADER_SIZE + metaBytes.length] = 0x0a;
    out.set(body, HEADER_SIZE + metaBytes.length + 1);
  }
  return out;
}

export type DecodedFrame = { meta: unknown; body: Uint8Array | null };

export class FrameDecoder {
  private buf = new Uint8Array(0);

  push(chunk: Uint8Array): DecodedFrame[] {
    const merged = new Uint8Array(this.buf.length + chunk.length);
    merged.set(this.buf, 0);
    merged.set(chunk, this.buf.length);
    this.buf = merged;

    const results: DecodedFrame[] = [];

    while (this.buf.length >= HEADER_SIZE) {
      const payloadLen =
        ((this.buf[0]! << 24) | (this.buf[1]! << 16) | (this.buf[2]! << 8) | this.buf[3]!) >>> 0;
      if (this.buf.length < HEADER_SIZE + payloadLen) break;

      const payload = this.buf.subarray(HEADER_SIZE, HEADER_SIZE + payloadLen);

      const nlIdx = payload.indexOf(0x0a);
      let meta: unknown;
      let body: Uint8Array | null = null;

      if (nlIdx !== -1) {
        meta = JSON.parse(decoder.decode(payload.subarray(0, nlIdx)));
        body = payload.slice(nlIdx + 1);
      } else {
        meta = JSON.parse(decoder.decode(payload));
      }

      results.push({ meta, body });
      this.buf = this.buf.slice(HEADER_SIZE + payloadLen);
    }

    return results;
  }
}
