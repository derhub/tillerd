const HEADER_SIZE = 4;

export function encodeFrame(meta: unknown, body?: Uint8Array): Buffer {
  const metaBytes = Buffer.from(JSON.stringify(meta), "utf8");
  const payloadLen = body ? metaBytes.length + 1 + body.length : metaBytes.length;
  const header = Buffer.allocUnsafe(HEADER_SIZE);
  header.writeUInt32BE(payloadLen, 0);
  if (body) {
    return Buffer.concat([header, metaBytes, Buffer.from([0x0a]), body]);
  }
  return Buffer.concat([header, metaBytes]);
}

export type DecodedFrame = { meta: unknown; body: Buffer | null };

export class FrameDecoder {
  private buf = Buffer.alloc(0);

  push(chunk: Uint8Array): DecodedFrame[] {
    this.buf = Buffer.concat([this.buf, chunk]);
    const results: DecodedFrame[] = [];

    while (this.buf.length >= HEADER_SIZE) {
      const payloadLen = this.buf.readUInt32BE(0);
      if (this.buf.length < HEADER_SIZE + payloadLen) break;

      const payload = this.buf.subarray(HEADER_SIZE, HEADER_SIZE + payloadLen);
      this.buf = this.buf.subarray(HEADER_SIZE + payloadLen);

      const nlIdx = payload.indexOf(0x0a);
      let meta: unknown;
      let body: Buffer | null = null;

      if (nlIdx !== -1) {
        meta = JSON.parse(payload.subarray(0, nlIdx).toString("utf8"));
        body = Buffer.from(payload.subarray(nlIdx + 1));
      } else {
        meta = JSON.parse(payload.toString("utf8"));
      }

      results.push({ meta, body });
    }

    return results;
  }
}
