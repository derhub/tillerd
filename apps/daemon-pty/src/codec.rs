//! Wire framing codec.

use serde::Serialize;

const HEADER_SIZE: usize = 4;
const BODY_SEP: u8 = 0x0a;

pub fn encode_frame<T: Serialize>(meta: &T, body: Option<&[u8]>) -> Vec<u8> {
    let meta_bytes = serde_json::to_vec(meta).expect("frame meta must serialize");
    encode_frame_raw(&meta_bytes, body)
}

pub fn encode_frame_raw(meta_bytes: &[u8], body: Option<&[u8]>) -> Vec<u8> {
    let payload_len = match body {
        Some(b) => meta_bytes.len() + 1 + b.len(),
        None => meta_bytes.len(),
    };
    let mut out = Vec::with_capacity(HEADER_SIZE + payload_len);
    out.extend_from_slice(&(payload_len as u32).to_be_bytes());
    out.extend_from_slice(meta_bytes);
    if let Some(b) = body {
        out.push(BODY_SEP);
        out.extend_from_slice(b);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub meta: Vec<u8>,
    pub body: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<DecodedFrame> {
        self.buf.extend_from_slice(chunk);
        let mut results = Vec::new();

        let mut offset = 0usize;
        while self.buf.len() - offset >= HEADER_SIZE {
            let len_bytes = [
                self.buf[offset],
                self.buf[offset + 1],
                self.buf[offset + 2],
                self.buf[offset + 3],
            ];
            let payload_len = u32::from_be_bytes(len_bytes) as usize;
            if self.buf.len() - offset < HEADER_SIZE + payload_len {
                break;
            }
            let start = offset + HEADER_SIZE;
            let end = start + payload_len;
            let payload = &self.buf[start..end];

            // Split on the FIRST 0x0a — JSON.stringify never emits a raw newline,
            // so the first newline is always the meta/body separator.
            let frame = match payload.iter().position(|&b| b == BODY_SEP) {
                Some(nl) => DecodedFrame {
                    meta: payload[..nl].to_vec(),
                    body: Some(payload[nl + 1..].to_vec()),
                },
                None => DecodedFrame {
                    meta: payload.to_vec(),
                    body: None,
                },
            };
            results.push(frame);
            offset = end;
        }

        if offset > 0 {
            self.buf.drain(..offset);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip_without_body() {
        let meta = json!({ "type": "hello", "versions": [1] });
        let encoded = encode_frame(&meta, None);
        let mut dec = FrameDecoder::new();
        let frames = dec.push(&encoded);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&frames[0].meta).unwrap(),
            meta
        );
        assert_eq!(frames[0].body, None);
    }

    #[test]
    fn round_trip_with_body() {
        let meta = json!({ "type": "data", "sessionId": "s1", "bodyLen": 5 });
        let body = [1u8, 2, 3, 4, 5];
        let encoded = encode_frame(&meta, Some(&body));
        let mut dec = FrameDecoder::new();
        let frames = dec.push(&encoded);
        assert_eq!(frames.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&frames[0].meta).unwrap(),
            meta
        );
        assert_eq!(frames[0].body.as_deref(), Some(&body[..]));
    }

    #[test]
    fn multiple_frames_in_one_push() {
        let mut combined = encode_frame(&json!({ "type": "ping" }), None);
        combined.extend(encode_frame(&json!({ "type": "pong" }), None));
        let mut dec = FrameDecoder::new();
        let frames = dec.push(&combined);
        assert_eq!(frames.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&frames[0].meta).unwrap()["type"],
            "ping"
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&frames[1].meta).unwrap()["type"],
            "pong"
        );
    }

    #[test]
    fn incomplete_frame_held_across_pushes() {
        let encoded = encode_frame(&json!({ "type": "hello" }), None);
        let half = encoded.len() / 2;
        let mut dec = FrameDecoder::new();
        assert_eq!(dec.push(&encoded[..half]).len(), 0);
        let second = dec.push(&encoded[half..]);
        assert_eq!(second.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&second[0].meta).unwrap()["type"],
            "hello"
        );
    }

    #[test]
    fn body_with_newline_bytes_preserved() {
        let body = [0x0au8, 0x0a, 0x41];
        let encoded = encode_frame(&json!({ "type": "data" }), Some(&body));
        let mut dec = FrameDecoder::new();
        let frames = dec.push(&encoded);
        assert_eq!(frames[0].body.as_deref(), Some(&body[..]));
    }

    /// Cross-implementation parity: bytes produced here must match the exact
    /// layout the reference `encodeFrame` produces for the same inputs.
    #[test]
    fn reference_byte_layout_parity() {
        // meta {"type":"x"} = 12 bytes; with body [0xff,0x00] => payload 12+1+2 = 15
        let encoded = encode_frame(&json!({ "type": "x" }), Some(&[0xff, 0x00]));
        let meta = br#"{"type":"x"}"#;
        let mut expected = Vec::new();
        expected.extend_from_slice(&(15u32).to_be_bytes());
        expected.extend_from_slice(meta);
        expected.push(0x0a);
        expected.extend_from_slice(&[0xff, 0x00]);
        assert_eq!(encoded, expected);
    }
}
