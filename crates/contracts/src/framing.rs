//! The length-prefixed frame codec shared by the loopback IPC faces and their
//! consumers. The wire is a 4-byte big-endian payload length followed by the
//! payload bytes.
//!
//! The codec is payload-agnostic: it frames opaque bytes and never inspects,
//! validates, or transforms them -- the payload encoding is the caller's concern.
//! It is also runtime-free: the async stream adapters (reading or writing one
//! frame over an async socket) stay with each face that owns a runtime, built on
//! the [`encode_frame`] and [`MAX_FRAME_SIZE`] defined here.

/// Bytes of the big-endian length prefix every frame carries.
pub const HEADER_SIZE: usize = 4;

/// The largest payload a single frame may carry. Enforced before allocation so a
/// hostile or corrupt length prefix cannot force a giant allocation.
pub const MAX_FRAME_SIZE: usize = 1 << 20;

/// Encode a length-prefixed frame: a 4-byte big-endian payload length, then the
/// payload bytes.
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// One complete frame's payload bytes. A frame carries only an opaque payload --
/// there is no raw body plane on this wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    /// The frame's payload bytes.
    pub payload: Vec<u8>,
}

/// A length prefix declared a payload larger than [`MAX_FRAME_SIZE`]. The decoder
/// rejects it before buffering toward the declared length.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("frame length {declared_len} exceeds max {}", MAX_FRAME_SIZE)]
pub struct OversizeFrame {
    /// The declared payload length that exceeded the maximum.
    pub declared_len: usize,
}

/// Incremental decoder: feed it socket chunks, get back every complete frame now
/// available. Holds a partial frame across pushes so a frame split over multiple
/// chunks is recovered once all its bytes arrive.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// A fresh decoder with an empty buffer.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Append `chunk` and return every complete frame now available. Returns
    /// [`OversizeFrame`] when a length prefix declares a payload larger than
    /// [`MAX_FRAME_SIZE`], rejecting it before buffering toward that length.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<RawFrame>, OversizeFrame> {
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
            if payload_len > MAX_FRAME_SIZE {
                return Err(OversizeFrame {
                    declared_len: payload_len,
                });
            }
            if self.buf.len() - offset < HEADER_SIZE + payload_len {
                break;
            }
            let start = offset + HEADER_SIZE;
            let end = start + payload_len;
            results.push(RawFrame {
                payload: self.buf[start..end].to_vec(),
            });
            offset = end;
        }

        if offset > 0 {
            self.buf.drain(..offset);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_round_trips_the_payload() {
        let mut decoder = FrameDecoder::new();
        let frames = decoder.push(&encode_frame(b"alpha")).unwrap();
        assert_eq!(
            frames,
            vec![RawFrame {
                payload: b"alpha".to_vec()
            }]
        );
    }

    #[test]
    fn multiple_frames_in_one_stream_decode_in_order() {
        let mut stream = encode_frame(b"alpha");
        stream.extend_from_slice(&encode_frame(b"beta"));

        let mut decoder = FrameDecoder::new();
        let frames = decoder.push(&stream).unwrap();
        assert_eq!(
            frames,
            vec![
                RawFrame {
                    payload: b"alpha".to_vec()
                },
                RawFrame {
                    payload: b"beta".to_vec()
                },
            ]
        );
    }

    #[test]
    fn a_frame_split_across_chunks_is_recovered() {
        let frame = encode_frame(b"split me");
        let (first, second) = frame.split_at(6);

        let mut decoder = FrameDecoder::new();
        assert_eq!(decoder.push(first).unwrap(), vec![]);
        assert_eq!(
            decoder.push(second).unwrap(),
            vec![RawFrame {
                payload: b"split me".to_vec()
            }]
        );
    }

    #[test]
    fn a_clean_chunk_boundary_yields_no_partial_frame() {
        let mut decoder = FrameDecoder::new();
        assert_eq!(decoder.push(&[]).unwrap(), vec![]);
    }

    #[test]
    fn an_oversize_length_prefix_is_rejected_before_allocation() {
        let oversize = (MAX_FRAME_SIZE as u32) + 1;
        let header = oversize.to_be_bytes();

        let mut decoder = FrameDecoder::new();
        let err = decoder.push(&header).unwrap_err();
        assert_eq!(err.declared_len, MAX_FRAME_SIZE + 1);
    }
}
