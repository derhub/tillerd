use std::task::{Context, Poll};

use tower::{Layer, Service};

use crate::app::notification::{PruneNotifications, RecordNotification};
use crate::context::Ctx;
use crate::shared::bus::{BoxFuture, Op};
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

/// Turns an observed `Notable` signal into exactly one recorded notification.
/// Ordinary commands and queries (where `Op::notable` is `None`, or no recorder
/// is composed) pass straight through. This is the single recording point.
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
        let to_record = self
            .recorder
            .as_ref()
            .and_then(|r| op.notable.take().map(|n| (r.clone(), n)));
        let fut = self.inner.call(op);
        Box::pin(async move {
            let out = fut.await?;
            if let Some((recorder, n)) = to_record {
                recorder.record(n).await;
            }
            Ok(out)
        })
    }
}
