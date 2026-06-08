//! Client-side codec for the gate hook-subscription wire.
//!
//! This crate is the Rust owner of the consumer side of the gate's subscribe
//! face. It mirrors the gate's server-side codec ([`apps/gate`]'s `endpoint`):
//! length-prefixed JSON frames (a 4-byte big-endian payload length, then a JSON
//! payload — no raw body plane), the [`HookSubscribeRequest`] a consumer sends,
//! and the typed handshake/event/error frames a consumer decodes.
//!
//! It carries no transport — a consumer drives its own socket and feeds bytes to
//! [`FrameDecoder`]. It imports neither the daemon PTY client nor memorya.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use contracts::{HookEvent, HookSubscribeRequest, HOOK_SUBSCRIPTION_WIRE_VERSION};
use serde_json::Value;

const HEADER_SIZE: usize = 4;

/// The gate hook-subscription wire version this client speaks (R9: sourced from
/// `contracts-rs`, negotiated with the gate at subscribe time).
pub const WIRE_VERSION: u32 = HOOK_SUBSCRIPTION_WIRE_VERSION;

/// One complete length-prefixed frame's JSON payload. The gate's subscribe face
/// carries only JSON, so a frame has no raw body plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    /// The frame's JSON payload bytes.
    pub payload: Vec<u8>,
}

/// Encode a frame the gate's subscribe face accepts: a 4-byte big-endian payload
/// length, then the payload. Mirrors the gate's `endpoint::encode_frame`.
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Encode the [`HookSubscribeRequest`] a consumer sends to open a stream.
pub fn encode_subscribe_request(request: &HookSubscribeRequest) -> Vec<u8> {
    encode_frame(&serde_json::to_vec(request).expect("subscribe request encodes"))
}

/// Incremental decoder: feed it socket chunks, get back complete frames. Holds a
/// partial frame across pushes.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// A fresh decoder with an empty buffer.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Append `chunk` and return every complete frame now available.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<RawFrame> {
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
            results.push(RawFrame {
                payload: self.buf[start..end].to_vec(),
            });
            offset = end;
        }

        if offset > 0 {
            self.buf.drain(..offset);
        }
        results
    }
}

/// A decoded subscribe-face frame. The gate keys every frame by a `frame`
/// discriminant: a `ready` handshake, an `event`, or an `error`.
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionFrame {
    /// The gate's handshake reply, carrying the negotiated wire version.
    Ready {
        /// Negotiated wire version.
        wire_version: u32,
    },
    /// A hook event for the subscribed session.
    Event(HookEvent),
    /// The gate refused or aborted the subscription.
    Error {
        /// Why the subscription was refused.
        reason: String,
    },
    /// A frame whose `frame` discriminant this client does not model
    /// (forward-compatible).
    Other {
        /// The unrecognized `frame` discriminant.
        frame: String,
    },
}

/// Decode a [`RawFrame`] into a typed [`SubscriptionFrame`]. Returns `None` when
/// the payload is not valid JSON, carries no string `frame`, or is an `event`
/// frame whose embedded hook event does not parse.
pub fn decode_subscription_frame(frame: &RawFrame) -> Option<SubscriptionFrame> {
    let meta: Value = serde_json::from_slice(&frame.payload).ok()?;
    let frame_kind = meta.get("frame")?.as_str()?;

    Some(match frame_kind {
        "ready" => SubscriptionFrame::Ready {
            wire_version: meta.get("wireVersion").and_then(Value::as_u64).unwrap_or(0) as u32,
        },
        "event" => {
            let event: HookEvent = serde_json::from_value(meta.get("event")?.clone()).ok()?;
            SubscriptionFrame::Event(event)
        }
        "error" => SubscriptionFrame::Error {
            reason: meta
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        },
        other => SubscriptionFrame::Other {
            frame: other.to_string(),
        },
    })
}

/// Why a subscription handshake could not be completed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecodeError {
    /// The gate negotiated a wire version this client does not speak.
    #[error("gate negotiated wire version {got}, client speaks {expected}")]
    WireVersionMismatch {
        /// Version this client speaks.
        expected: u32,
        /// Version the gate offered.
        got: u32,
    },
    /// The gate refused the subscription with a reason.
    #[error("gate refused subscription: {reason}")]
    Rejected {
        /// The gate-supplied refusal reason.
        reason: String,
    },
    /// The gate's opening frame was an event or unmodeled frame, not a handshake.
    #[error("expected a ready handshake, got a different frame")]
    UnexpectedFrame,
}

/// Verify the gate's opening frame negotiates a wire version this client speaks,
/// returning the negotiated version. The transport (reading the frame) is the
/// caller's; this is the pure protocol decision.
pub fn negotiate_ready(frame: &SubscriptionFrame) -> Result<u32, DecodeError> {
    match frame {
        SubscriptionFrame::Ready { wire_version } if *wire_version == WIRE_VERSION => {
            Ok(*wire_version)
        }
        SubscriptionFrame::Ready { wire_version } => Err(DecodeError::WireVersionMismatch {
            expected: WIRE_VERSION,
            got: *wire_version,
        }),
        SubscriptionFrame::Error { reason } => Err(DecodeError::Rejected {
            reason: reason.clone(),
        }),
        SubscriptionFrame::Event(_) | SubscriptionFrame::Other { .. } => {
            Err(DecodeError::UnexpectedFrame)
        }
    }
}
