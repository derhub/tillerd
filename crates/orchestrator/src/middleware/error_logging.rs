use std::error::Error as _;
use std::task::{Context, Poll};

use tower::{Layer, Service};
use tracing::Instrument;

use crate::shared::bus::{BoxFuture, Op, OpKind};
use crate::shared::Error;

/// Opens the per-operation span and, on `Err`, emits the single structured
/// `ERROR` event with OTel-named fields. Passes `Ok(T)` through untouched.
pub(crate) struct ErrorLoggingLayer;

impl<S> Layer<S> for ErrorLoggingLayer {
    type Service = ErrorLogging<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ErrorLogging { inner }
    }
}

pub(crate) struct ErrorLogging<S> {
    inner: S,
}

impl<S, T> Service<Op<T>> for ErrorLogging<S>
where
    S: Service<Op<T>, Response = T, Error = Error>,
    S::Future: Send + 'static,
    T: 'static,
{
    type Response = T;
    type Error = Error;
    type Future = BoxFuture<T>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, op: Op<T>) -> Self::Future {
        let span = match op.kind {
            OpKind::Command => tracing::info_span!("command", action = op.action),
            OpKind::Query => tracing::info_span!("query", action = op.action),
        };
        let fut = self.inner.call(op);
        Box::pin(async move { fut.instrument(span).await.inspect_err(record) })
    }
}

/// One structured `ERROR` event with OTel-named fields. The stable `code()` is
/// the low-cardinality `error.type`; the id stays in the message, not the code.
fn record(e: &Error) {
    tracing::error!(
        error.type = e.code(),
        exception.message = %e,
        source = ?e.source(),
    );
}
