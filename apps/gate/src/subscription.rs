//! Per-session hook-event pub/sub. Each session owns a bounded broadcast channel
//! that drops the oldest event when a consumer falls behind, plus the
//! `HOOK_SUBSCRIPTION_WIRE_VERSION`-gated frame codec the subscribe face streams.

use std::collections::HashMap;
use std::sync::Mutex;

use contracts::{HookEvent, SessionId, HOOK_SUBSCRIPTION_WIRE_VERSION};
use serde_json::json;
use tokio::sync::broadcast;

/// The subscription queue depth when the override is unset.
const DEFAULT_QUEUE_CAP: usize = 256;

/// Environment override for the per-session queue depth.
const QUEUE_CAP_ENV: &str = "ATHING_GATE_QUEUE_CAP";

/// The gate hook-subscription wire version (R9: sourced from `contracts-rs`,
/// independent of the daemon session-event wire).
pub const SUBSCRIPTION_WIRE_VERSION: u32 = HOOK_SUBSCRIPTION_WIRE_VERSION;

/// Why an event could not be delivered to any consumer.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PublishError {
    /// The session has no active subscribers (normal under fire-and-forget).
    #[error("no active subscribers")]
    NoSubscribers,
}

struct Channel {
    sender: broadcast::Sender<HookEvent>,
    dropped: u64,
}

/// The per-session pub/sub registry: a bounded, drop-oldest broadcast channel
/// per subscribed session plus its dropped-event counter.
pub struct Subscriptions {
    capacity: usize,
    channels: Mutex<HashMap<SessionId, Channel>>,
}

impl Default for Subscriptions {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAP)
    }
}

impl Subscriptions {
    /// A registry with the default queue depth.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry that reads its queue depth from the environment override.
    pub fn from_env() -> Self {
        Self::with_capacity(resolve_capacity(
            std::env::var(QUEUE_CAP_ENV).ok().as_deref(),
        ))
    }

    /// A registry with an explicit queue depth (clamped to at least one, since a
    /// zero-capacity broadcast channel is invalid).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// The per-session queue depth this registry creates channels with.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Subscribe to a session's hook-event stream, creating the channel on first
    /// use. The receiver observes only events published after it subscribes.
    pub fn subscribe(&self, session: &SessionId) -> broadcast::Receiver<HookEvent> {
        let mut guard = self.channels.lock().expect("subscriptions mutex poisoned");
        let channel = guard.entry(session.clone()).or_insert_with(|| Channel {
            sender: broadcast::channel(self.capacity).0,
            dropped: 0,
        });
        channel.sender.subscribe()
    }

    /// Publish an event to a session's subscribers, returning the number reached.
    /// Never blocks: a full buffer drops its oldest event natively.
    pub fn publish(&self, session: &SessionId, event: HookEvent) -> Result<usize, PublishError> {
        let guard = self.channels.lock().expect("subscriptions mutex poisoned");
        let channel = guard.get(session).ok_or(PublishError::NoSubscribers)?;
        channel
            .sender
            .send(event)
            .map_err(|_| PublishError::NoSubscribers)
    }

    /// End a session's stream: drops the sender so every subscriber observes the
    /// channel close once it has drained.
    pub fn end(&self, session: &SessionId) {
        self.channels
            .lock()
            .expect("subscriptions mutex poisoned")
            .remove(session);
    }

    /// End every session's stream. Run at shutdown so all subscribers observe the
    /// channel close as the gate drains.
    pub fn clear(&self) {
        self.channels
            .lock()
            .expect("subscriptions mutex poisoned")
            .clear();
    }

    /// Record a consumer-observed lag: add the skipped count to the session's
    /// dropped total and log the loss once.
    pub fn record_lag(&self, session: &SessionId, skipped: u64) {
        let mut guard = self.channels.lock().expect("subscriptions mutex poisoned");
        if let Some(channel) = guard.get_mut(session) {
            channel.dropped += skipped;
            tracing::warn!(
                session = %session.0,
                skipped,
                dropped_n = channel.dropped,
                "hook subscription lagged; dropped oldest events"
            );
        }
    }

    /// The total events dropped for a session.
    pub fn dropped(&self, session: &SessionId) -> u64 {
        self.channels
            .lock()
            .expect("subscriptions mutex poisoned")
            .get(session)
            .map_or(0, |channel| channel.dropped)
    }
}

/// Negotiate the subscription wire version: v1 accepts only an exact match.
pub fn negotiate(requested: u32) -> Option<u32> {
    (requested == SUBSCRIPTION_WIRE_VERSION).then_some(SUBSCRIPTION_WIRE_VERSION)
}

/// Encode the handshake frame that opens a stream, carrying the negotiated wire
/// version.
pub fn encode_ready() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "frame": "ready",
        "wireVersion": SUBSCRIPTION_WIRE_VERSION,
    }))
    .expect("ready frame encodes")
}

/// Encode a hook-event frame for the stream.
pub fn encode_event(event: &HookEvent) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "frame": "event",
        "event": event,
    }))
    .expect("event frame encodes")
}

fn resolve_capacity(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_QUEUE_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::{CorrelationId, HookKind};
    use serde_json::Value;
    use std::time::Duration;
    use tokio::sync::broadcast::error::{RecvError, TryRecvError};

    fn session(id: &str) -> SessionId {
        SessionId(id.into())
    }

    fn event(correlation: &str) -> HookEvent {
        HookEvent {
            session_id: session("s"),
            correlation_id: CorrelationId(correlation.into()),
            ts: 0,
            kind: HookKind::Stop { turn_index: None },
        }
    }

    #[test]
    fn new_subscriptions_default_to_a_capacity_of_256() {
        assert_eq!(Subscriptions::new().capacity(), 256);
    }

    #[test]
    fn capacity_defaults_when_the_override_is_absent() {
        assert_eq!(resolve_capacity(None), 256);
    }

    #[test]
    fn capacity_uses_a_valid_override() {
        assert_eq!(resolve_capacity(Some("512")), 512);
    }

    #[test]
    fn capacity_falls_back_when_the_override_is_zero() {
        assert_eq!(resolve_capacity(Some("0")), 256);
    }

    #[test]
    fn capacity_falls_back_when_the_override_is_not_a_number() {
        assert_eq!(resolve_capacity(Some("lots")), 256);
    }

    #[tokio::test]
    async fn publish_delivers_to_every_subscriber() {
        let subs = Subscriptions::with_capacity(8);
        let s = session("s");
        let mut a = subs.subscribe(&s);
        let mut b = subs.subscribe(&s);
        let mut c = subs.subscribe(&s);

        let reached = subs.publish(&s, event("e1")).unwrap();

        assert_eq!(reached, 3);
        for rx in [&mut a, &mut b, &mut c] {
            assert_eq!(
                rx.recv().await.unwrap().correlation_id,
                CorrelationId("e1".into())
            );
        }
    }

    #[tokio::test]
    async fn publish_is_isolated_per_session() {
        let subs = Subscriptions::with_capacity(8);
        let a = session("a");
        let b = session("b");
        let mut rx_a = subs.subscribe(&a);
        let mut rx_b = subs.subscribe(&b);

        subs.publish(&a, event("only-a")).unwrap();

        assert_eq!(
            rx_a.recv().await.unwrap().correlation_id,
            CorrelationId("only-a".into())
        );
        assert!(matches!(rx_b.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn publish_drops_the_oldest_event_when_the_buffer_is_full() {
        let subs = Subscriptions::with_capacity(2);
        let s = session("s");
        let mut rx = subs.subscribe(&s);
        for i in 1..=4 {
            let _ = subs.publish(&s, event(&format!("e{i}")));
        }

        assert!(
            matches!(rx.recv().await, Err(RecvError::Lagged(_))),
            "the slow receiver lags"
        );
        let mut delivered = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            delivered.push(ev.correlation_id.0);
        }

        assert!(
            delivered.iter().any(|c| c == "e4"),
            "the newest event is retained"
        );
        assert!(
            !delivered.iter().any(|c| c == "e1"),
            "the oldest event was dropped"
        );
    }

    #[tokio::test]
    async fn publish_never_blocks_when_the_buffer_is_full() {
        let subs = Subscriptions::with_capacity(2);
        let s = session("s");
        let _rx = subs.subscribe(&s);

        let outcome = tokio::time::timeout(Duration::from_millis(500), async {
            for i in 0..1000 {
                let _ = subs.publish(&s, event(&i.to_string()));
            }
        })
        .await;

        assert!(
            outcome.is_ok(),
            "publishing to a full buffer must never block the poster"
        );
    }

    #[tokio::test]
    async fn observing_lag_increments_the_dropped_counter() {
        let subs = Subscriptions::with_capacity(2);
        let s = session("s");
        let mut rx = subs.subscribe(&s);
        for i in 0..5 {
            let _ = subs.publish(&s, event(&i.to_string()));
        }

        let skipped = match rx.recv().await {
            Err(RecvError::Lagged(n)) => n,
            other => panic!("expected a lag, got {other:?}"),
        };
        subs.record_lag(&s, skipped);

        assert_eq!(subs.dropped(&s), skipped);
    }

    #[tokio::test]
    async fn ending_a_session_closes_its_subscribers() {
        let subs = Subscriptions::with_capacity(8);
        let s = session("s");
        let mut rx = subs.subscribe(&s);

        subs.end(&s);

        assert!(matches!(rx.recv().await, Err(RecvError::Closed)));
    }

    #[tokio::test]
    async fn clearing_closes_every_session_stream() {
        let subs = Subscriptions::with_capacity(8);
        let mut a = subs.subscribe(&session("a"));
        let mut b = subs.subscribe(&session("b"));

        subs.clear();

        assert!(matches!(a.recv().await, Err(RecvError::Closed)));
        assert!(matches!(b.recv().await, Err(RecvError::Closed)));
    }

    #[tokio::test]
    async fn publishing_without_subscribers_reports_no_subscribers() {
        let subs = Subscriptions::with_capacity(8);

        assert_eq!(
            subs.publish(&session("ghost"), event("e")),
            Err(PublishError::NoSubscribers)
        );
    }

    #[test]
    fn ready_frame_carries_the_subscription_wire_version() {
        let frame: Value = serde_json::from_slice(&encode_ready()).unwrap();
        assert_eq!(
            frame,
            json!({ "frame": "ready", "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION })
        );
    }

    #[test]
    fn event_frame_carries_the_hook_event() {
        let original = event("e1");
        let frame: Value = serde_json::from_slice(&encode_event(&original)).unwrap();

        assert_eq!(frame["frame"], "event");
        let decoded: HookEvent = serde_json::from_value(frame["event"].clone()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn negotiate_accepts_the_supported_version() {
        assert_eq!(
            negotiate(HOOK_SUBSCRIPTION_WIRE_VERSION),
            Some(SUBSCRIPTION_WIRE_VERSION)
        );
    }

    #[test]
    fn negotiate_rejects_an_unsupported_version() {
        assert_eq!(negotiate(HOOK_SUBSCRIPTION_WIRE_VERSION + 1), None);
    }
}
