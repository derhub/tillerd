//! Owns the dispatch middleware stack: the single place that declares which
//! tower layers wrap the bus handler and in what order.

mod error_logging;
mod notification_recording;

use tower::{Service, ServiceBuilder};

use error_logging::ErrorLoggingLayer;
use notification_recording::NotificationRecordingLayer;

pub(crate) use notification_recording::NotificationRecorder;

use crate::shared::bus::{BoxFuture, HandlerService, Op};
use crate::shared::Error;

/// Compose the production middleware stack around the handler. This is the one
/// place that fixes the layer order: `ErrorLoggingLayer` -> `NotificationRecordingLayer`
/// -> `HandlerService`. The recording layer is a pass-through when `recorder` is
/// `None` (the ordinary command/query path).
pub(crate) fn pipeline<T>(
    recorder: Option<NotificationRecorder>,
) -> impl Service<Op<T>, Response = T, Error = Error, Future = BoxFuture<T>>
where
    T: Send + 'static,
{
    ServiceBuilder::new()
        .layer(ErrorLoggingLayer)
        .layer(NotificationRecordingLayer { recorder })
        .service(HandlerService)
}
