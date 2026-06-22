//! The generic dispatcher. `Bus<Cx>` is a thin pass-through over a context: it
//! constructs a span per operation and, on error, emits one structured `ERROR`
//! event with OTel-named fields -- and nothing else. It does NOT own a
//! transaction (that is each command's concern) and it never boxes (dispatch is
//! static over the concrete operation type).
//!
//! Surface input/resize/attach never pass through the bus, so no keystroke
//! payload is ever captured by a span or event here.

use std::error::Error as _;

use tracing::Instrument;

use crate::shared::message::{Command, Query};
use crate::shared::{Error, Result};

/// Dispatches commands and queries over a shared context, carrying only the
/// cross-cutting telemetry. External I/O lives off the bus (the runtime port and
/// its sink), so raw payloads are never captured on the telemetry path.
pub struct Bus<Cx> {
    cx: Cx,
}

impl<Cx> Bus<Cx> {
    pub fn new(cx: Cx) -> Self {
        Self { cx }
    }

    /// Borrow the context (for transports that build values needing it).
    pub fn cx(&self) -> &Cx {
        &self.cx
    }

    /// Run a mutation. Returns its `Result<()>`; on error, logs one `ERROR`
    /// event. No transaction is opened here.
    pub async fn execute<C: Command<Cx>>(&self, c: C) -> Result<()> {
        let span = tracing::info_span!("command", action = std::any::type_name::<C>());
        c.handle(&self.cx)
            .instrument(span)
            .await
            .inspect_err(record)
    }

    /// Run a read. Returns its `Out`; on error, logs one `ERROR` event. Reads
    /// never hold a write lock.
    pub async fn query<Q: Query<Cx>>(&self, q: Q) -> Result<Q::Out> {
        let span = tracing::info_span!("query", action = std::any::type_name::<Q>());
        q.handle(&self.cx)
            .instrument(span)
            .await
            .inspect_err(record)
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    use super::*;

    struct Out;
    impl Command<()> for Out {
        async fn handle(&self, _cx: &()) -> Result<()> {
            Ok(())
        }
    }

    struct FailingCommand;
    impl Command<()> for FailingCommand {
        async fn handle(&self, _cx: &()) -> Result<()> {
            Err(Error::WorkspaceNotFound("ws_9f3c".to_owned()))
        }
    }

    struct DoubleQuery(u32);
    impl Query<()> for DoubleQuery {
        type Out = u32;
        async fn handle(&self, _cx: &()) -> Result<u32> {
            Ok(self.0 * 2)
        }
    }

    struct FailingQuery;
    impl Query<()> for FailingQuery {
        type Out = u32;
        async fn handle(&self, _cx: &()) -> Result<u32> {
            Err(Error::ProjectNotFound("pr_1".to_owned()))
        }
    }

    /// A passthrough command whose context is a shared cell it writes through, to
    /// prove the bus hands the command the context.
    struct Increment;
    impl Command<Mutex<u32>> for Increment {
        async fn handle(&self, cx: &Mutex<u32>) -> Result<()> {
            *cx.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    struct ErrorEvents(Arc<Mutex<Vec<String>>>);

    struct CodeVisitor<'a>(&'a mut Option<String>);
    impl Visit for CodeVisitor<'_> {
        fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "error.type" {
                *self.0 = Some(value.to_owned());
            }
        }
    }

    impl<S: Subscriber> Layer<S> for ErrorEvents {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            if *event.metadata().level() != Level::ERROR {
                return;
            }
            let mut code = None;
            event.record(&mut CodeVisitor(&mut code));
            if let Some(code) = code {
                self.0.lock().unwrap().push(code);
            }
        }
    }

    #[tokio::test]
    async fn execute_returns_ok_and_passes_the_context_to_the_command() {
        let bus = Bus::new(Mutex::new(0));
        bus.execute(Increment).await.unwrap();
        assert_eq!(*bus.cx().lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn query_returns_the_handlers_output() {
        let bus = Bus::new(());
        assert_eq!(bus.query(DoubleQuery(21)).await.unwrap(), 42);
    }

    #[tokio::test]
    async fn execute_propagates_the_command_error() {
        let bus = Bus::new(());
        let err = bus.execute(FailingCommand).await.unwrap_err();
        assert_eq!(err.code(), "workspace.not_found");
    }

    #[tokio::test]
    async fn query_propagates_the_handler_error() {
        let bus = Bus::new(());
        let err = bus.query(FailingQuery).await.unwrap_err();
        assert_eq!(err.code(), "project.not_found");
    }

    #[tokio::test]
    async fn a_command_error_logs_exactly_one_error_event_with_the_stable_code() {
        let events = ErrorEvents::default();
        let _guard = tracing_subscriber::registry()
            .with(events.clone())
            .set_default();

        let bus = Bus::new(());
        let _ = bus.execute(FailingCommand).await;

        let captured = events.0.lock().unwrap();
        assert_eq!(captured.as_slice(), ["workspace.not_found"]);
    }

    #[tokio::test]
    async fn a_successful_operation_logs_no_error_event() {
        let events = ErrorEvents::default();
        let _guard = tracing_subscriber::registry()
            .with(events.clone())
            .set_default();

        let bus = Bus::new(());
        bus.execute(Out).await.unwrap();

        assert!(events.0.lock().unwrap().is_empty());
    }
}
