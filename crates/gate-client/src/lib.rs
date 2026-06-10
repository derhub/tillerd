//! Gate hook-subscription decoder (Rust consumer side). Consumer provides transport.
//! Framing reuses the shared contracts::framing codec; no daemon-pty or memorya imports.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use contracts::{HookEvent, Route, RoutePreamble, SessionId, HOOK_SUBSCRIPTION_WIRE_VERSION};
use serde_json::Value;

pub use contracts::framing::{encode_frame, FrameDecoder, OversizeFrame, RawFrame};

/// The gate wire version this client speaks (R9: sourced from `contracts-rs`,
/// carried in the route preamble and negotiated with the gate at subscribe time).
pub const WIRE_VERSION: u32 = HOOK_SUBSCRIPTION_WIRE_VERSION;

/// Encode the route preamble a consumer sends to open a `Subscribe` stream on the
/// gate's single socket. The subscribe route carries no token.
pub fn encode_subscribe_preamble(session_id: &SessionId) -> Vec<u8> {
    let preamble = RoutePreamble {
        route: Route::Subscribe,
        session_id: Some(session_id.clone()),
        token: None,
        wire_version: WIRE_VERSION,
    };
    encode_frame(&serde_json::to_vec(&preamble).expect("subscribe preamble encodes"))
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
