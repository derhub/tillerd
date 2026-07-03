use std::task::{Context, Poll};

use tower::{Layer, Service};

use crate::app::notification::{PruneNotifications, RecordNotification};
use crate::context::Ctx;
use crate::shared::bus::{BoxFuture, Op, OpKind};
use crate::shared::message::Command;
use crate::shared::Error;

/// Durable-history retention: the recording layer keeps the most recent N
/// notifications, pruning older ones after each insert.
const NOTIFICATION_HISTORY: u32 = 500;

/// The notification-recording dependency wired in from `Ctx`: the store the
/// notification is persisted to, plus the change sink the host subscribes to for
/// live UI push. The host cannot observe the store directly, so a recorded
/// notification is announced through `Ctx`'s notifications-changed broadcast.
#[derive(Clone)]
pub(crate) struct NotificationRecorder {
    ctx: Ctx,
}

impl NotificationRecorder {
    pub(crate) fn new(ctx: Ctx) -> Self {
        Self { ctx }
    }

    /// Persist one notification, prune to the retention cap, then announce the
    /// change. Best-effort: a store error never propagates onto the signal's
    /// dispatch result.
    async fn record(&self, n: RecordNotification) {
        if RecordNotification::handle(&n, &self.ctx).await.is_ok() {
            let _ = PruneNotifications {
                keep: NOTIFICATION_HISTORY,
            }
            .handle(&self.ctx)
            .await;
            self.ctx.notifications_changed().dispatch(|s| s.emit(&n));
        }
    }
}

/// Build the notification for a failed user-initiated mutation. Category
/// `command-error`; the action's type name rides in `detail` for diagnosis.
fn failure_notification(action: &'static str, error: &Error) -> RecordNotification {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    RecordNotification {
        id: uuid::Uuid::new_v4().to_string(),
        category: "command-error".to_owned(),
        severity: "error".to_owned(),
        title: None,
        message: error.to_string(),
        detail: Some(action.to_owned()),
        ts,
        session_id: None,
        surface_id: None,
        actions_json: None,
        read: false,
        snooze_until: None,
    }
}

/// Turns an observed `Notable` signal into exactly one recorded notification, and
/// records a `command-error` notification when a recorder-composed mutation fails
/// -- the orchestrator is the sole recording point (the renderer never records).
/// Ordinary commands and queries (where `Op::notable` is `None` and no recorder
/// is composed) pass straight through.
pub(crate) struct NotificationRecordingLayer {
    pub(crate) recorder: Option<NotificationRecorder>,
}

impl<S> Layer<S> for NotificationRecordingLayer {
    type Service = NotificationRecording<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NotificationRecording {
            inner,
            recorder: self.recorder.clone(),
        }
    }
}

pub(crate) struct NotificationRecording<S> {
    inner: S,
    recorder: Option<NotificationRecorder>,
}

impl<S, T> Service<Op<T>> for NotificationRecording<S>
where
    S: Service<Op<T>, Response = T, Error = Error>,
    S::Future: Send + 'static,
    T: Send + 'static,
{
    type Response = T;
    type Error = Error;
    type Future = BoxFuture<T>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut op: Op<T>) -> Self::Future {
        let recorder = self.recorder.clone();
        let notable = op.notable.take();
        let action = op.action;
        let is_command = matches!(op.kind, OpKind::Command);
        let fut = self.inner.call(op);
        Box::pin(async move {
            match fut.await {
                Ok(out) => {
                    if let (Some(recorder), Some(n)) = (recorder, notable) {
                        recorder.record(n).await;
                    }
                    Ok(out)
                }
                Err(e) => {
                    if let (Some(recorder), true) = (recorder, is_command) {
                        recorder.record(failure_notification(action, &e)).await;
                    }
                    Err(e)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use crate::app::notification::{ListNotifications, RecordNotification};
    use crate::app::workspace::DiscardWorkspace;
    use crate::shared::bus::Bus;
    use crate::shared::message::Query;

    // Scenario: A guard-rejected mutation is recorded by the orchestrator (the
    // renderer never records) and announced on the notifications-changed sink.
    #[tokio::test]
    async fn failed_recorded_command_records_a_command_error_notification() {
        let cx = crate::boot::test_ctx().await.unwrap();
        let announced = Arc::new(AtomicUsize::new(0));
        let seen = announced.clone();
        cx.notifications_changed()
            .subscribe(Arc::new(move |_: &RecordNotification| {
                seen.fetch_add(1, Ordering::SeqCst);
            }));
        let bus = Bus::new(cx.clone());

        // Discarding the Default workspace trips guard_not_default.
        let result = bus
            .execute_recorded(DiscardWorkspace {
                id: crate::app::workspace::default_workspace_id(),
            })
            .await;
        assert!(result.is_err(), "the guard must reject the discard");

        let listing = ListNotifications {
            limit: Some(10),
            offset: Some(0),
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        let recorded = listing
            .items
            .iter()
            .find(|n| n.category == "command-error")
            .expect("a command-error notification is persisted");
        assert_eq!(recorded.severity, "error");
        assert!(
            recorded
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("DiscardWorkspace"),
            "detail names the failed action"
        );
        assert_eq!(
            announced.load(Ordering::SeqCst),
            1,
            "announced exactly once"
        );
    }

    // Scenario: A successful recorded mutation records nothing.
    #[tokio::test]
    async fn successful_recorded_command_records_nothing() {
        let cx = crate::boot::test_ctx().await.unwrap();
        let bus = Bus::new(cx.clone());

        bus.execute_recorded(crate::app::workspace::NewWorkspaceCmd {
            id: "ws-rec-ok".to_owned(),
            name: "Ok".to_owned(),
        })
        .await
        .unwrap();

        let listing = ListNotifications {
            limit: Some(10),
            offset: Some(0),
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(
            listing.items.iter().all(|n| n.category != "command-error"),
            "no command-error notification for a success"
        );
    }
}
