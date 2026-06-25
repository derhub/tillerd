//! Notifications-changed contract: a sink the host subscribes to so a recorded
//! notification reaches the renderer as a live push. The recording layer owns
//! the store write; this sink only announces the recorded record so the host
//! never re-records it.

use crate::app::notification::RecordNotification;
use crate::shared::bus::Broadcast;

/// Receives the notification a recording-layer write just persisted. The host
/// converts it to its wire shape and pushes it to the renderer.
///
/// Implementations must be `Send + Sync + 'static` so they can be held behind
/// `Arc<dyn NotificationSink>` and shared across tasks. The method is
/// synchronous: implementations must not block.
pub trait NotificationSink: Send + Sync + 'static {
    fn emit(&self, notification: &RecordNotification);
}

/// `Broadcast<dyn NotificationSink>` is itself a `NotificationSink`: calling
/// `emit` dispatches to every registered subscriber synchronously.
impl NotificationSink for Broadcast<dyn NotificationSink> {
    fn emit(&self, notification: &RecordNotification) {
        self.dispatch(|s| s.emit(notification));
    }
}

/// Any `Fn(&RecordNotification) + Send + Sync + 'static` is a sink, so callers
/// can subscribe with a closure and need no explicit struct or impl.
impl<F> NotificationSink for F
where
    F: Fn(&RecordNotification) + Send + Sync + 'static,
{
    fn emit(&self, notification: &RecordNotification) {
        self(notification)
    }
}
