//! The generic dispatcher. `Bus<Cx>` re-expresses dispatch as a `tower::Service`
//! pipeline: each `execute`/`query` builds a one-shot operation envelope (`Op`)
//! whose handler future is pre-bound at the typed boundary, then drives it
//! through the middleware stack composed in `crate::middleware`. The
//! cross-cutting span and the single structured `ERROR` event live in the layer,
//! not inline. Handlers stay plain `Command<Cx>`/`Query<Cx>`; they never become
//! `Service`s. The one boxed handler future per dispatch is the only allocation.
//!
//! Surface input/resize/attach never pass through the bus, so no keystroke
//! payload is ever captured by a span or event here.
//!
//! `Broadcast<S: ?Sized>` is the companion fan-out primitive used by the event
//! dispatch layer (`events/`). It is separate from `Bus`: it carries no context,
//! no telemetry, and no CQS contract -- it is only a thread-safe subscriber list
//! with synchronous, borrow-and-forward iteration.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};

use tower::Service;

use crate::app::notification::RecordNotification;
use crate::context::Ctx;
use crate::middleware::{self, NotificationRecorder};
use crate::shared::message::{Command, Query};
use crate::shared::{Error, Result};

pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// A bus operation that, when dispatched, also yields a notification to record.
/// The recording layer reads `notification()` after the handler runs; `None`
/// means the operation is not notification-worthy. Only lifecycle signals impl
/// this -- ordinary commands and queries never do.
pub trait Notable {
    fn notification(&self) -> Option<RecordNotification>;
}

/// Thread-safe, synchronous 1:N event fan-out.
///
/// Each domain in `events/` pairs a borrowed-enum event type with a sink trait
/// and a `Broadcast<dyn SinkTrait>` instance that fans out to every subscriber.
/// `subscribe` registers an `Arc<S>` sink; `dispatch` iterates the list and
/// calls a closure over each, forwarding the borrowed event payload zero-copy.
pub struct Broadcast<S: ?Sized> {
    subs: RwLock<Vec<Arc<S>>>,
}

impl<S: ?Sized> Default for Broadcast<S> {
    fn default() -> Self {
        Self {
            subs: RwLock::new(Vec::new()),
        }
    }
}

impl<S: ?Sized> Broadcast<S> {
    /// Register a subscriber. Subscribers are called in registration order.
    pub fn subscribe(&self, sink: Arc<S>) {
        self.subs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(sink);
    }

    /// Synchronously call `f` for every registered subscriber. No-op when the
    /// list is empty; the read lock is held for the full iteration.
    pub fn dispatch(&self, f: impl Fn(&S)) {
        for s in self.subs.read().unwrap_or_else(|e| e.into_inner()).iter() {
            f(&**s);
        }
    }
}

/// Handle to one registration in a [`Registry`]. Pass it to
/// [`Registry::remove`] to tear down exactly that sink.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SinkId(u64);

/// Thread-safe, synchronous, key-scoped 1:N event fan-out.
///
/// Like [`Broadcast`] but partitioned by a `String` key: `register` adds an
/// `Arc<S>` sink under a key and returns a [`SinkId`]; `dispatch` calls a
/// closure over only the sinks registered for one key, forwarding the borrowed
/// event payload zero-copy; `remove` drops a single sink by its handle without
/// affecting the others. Registration and removal are safe concurrently with an
/// in-flight dispatch.
type KeyedSinks<S> = HashMap<String, Vec<(SinkId, Arc<S>)>>;

pub struct Registry<S: ?Sized> {
    sinks: RwLock<KeyedSinks<S>>,
    next: AtomicU64,
}

impl<S: ?Sized> Default for Registry<S> {
    fn default() -> Self {
        Self {
            sinks: RwLock::new(HashMap::new()),
            next: AtomicU64::new(0),
        }
    }
}

impl<S: ?Sized> Registry<S> {
    /// Register `sink` under `key`. Sinks under one key are called in
    /// registration order. The returned [`SinkId`] tears this sink down.
    pub fn register(&self, key: &str, sink: Arc<S>) -> SinkId {
        let id = SinkId(self.next.fetch_add(1, Ordering::Relaxed));
        self.sinks
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key.to_owned())
            .or_default()
            .push((id, sink));
        id
    }

    /// Remove the sink registered under `key` with handle `id`. No-op when the
    /// key or handle is unknown.
    pub fn remove(&self, key: &str, id: SinkId) {
        let mut sinks = self.sinks.write().unwrap_or_else(|e| e.into_inner());
        if let Some(entries) = sinks.get_mut(key) {
            entries.retain(|(existing, _)| *existing != id);
            if entries.is_empty() {
                sinks.remove(key);
            }
        }
    }

    /// Remove every sink registered under `key`. No-op when the key is unknown.
    /// Used when one client owns the whole key (one channel per surface) and
    /// teardown drops the key wholesale.
    pub fn remove_key(&self, key: &str) {
        self.sinks
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
    }

    /// Synchronously call `f` for every sink registered under `key`. No-op when
    /// the key has no sinks; the read lock is held for the full iteration.
    pub fn dispatch(&self, key: &str, f: impl Fn(&S)) {
        let sinks = self.sinks.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entries) = sinks.get(key) {
            for (_, s) in entries {
                f(&**s);
            }
        }
    }

    pub fn dispatch_prefix(&self, prefix: &str, f: impl Fn(&S)) {
        let sinks = self.sinks.read().unwrap_or_else(|e| e.into_inner());
        for (key, entries) in sinks.iter() {
            if key.starts_with(prefix) {
                for (_, s) in entries {
                    f(&**s);
                }
            }
        }
    }
}

/// Dispatches commands and queries over a shared context, carrying only the
/// cross-cutting telemetry. External I/O lives off the bus (the runtime port and
/// its sink), so raw payloads are never captured on the telemetry path.
pub struct Bus<Cx> {
    cx: Cx,
}

impl<Cx: Clone + Send + Sync + 'static> Bus<Cx> {
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
        let cx = self.cx.clone();
        let op = Op {
            action: std::any::type_name::<C>(),
            kind: OpKind::Command,
            notable: None,
            fut: Box::pin(async move { c.handle(&cx).await }),
        };
        drive(op, None).await
    }

    /// Run a read. Returns its `Out`; on error, logs one `ERROR` event. Reads
    /// never hold a write lock.
    pub async fn query<Q: Query<Cx>>(&self, q: Q) -> Result<Q::Out> {
        let cx = self.cx.clone();
        let op = Op {
            action: std::any::type_name::<Q>(),
            kind: OpKind::Query,
            notable: None,
            fut: Box::pin(async move { q.handle(&cx).await }),
        };
        drive(op, None).await
    }
}

impl Bus<Ctx> {
    /// Run a notification-worthy lifecycle signal through the bus. The signal's
    /// handler does its (minimal) domain work; the notification-recording layer
    /// then records exactly one notification from `c.notification()` and nudges
    /// the change sink. This is the single recording point.
    pub async fn execute_notable<C: Command<Ctx> + Notable>(&self, c: C) -> Result<()> {
        let cx = self.cx.clone();
        let op = Op {
            action: std::any::type_name::<C>(),
            kind: OpKind::Command,
            notable: c.notification(),
            fut: Box::pin(async move { c.handle(&cx).await }),
        };
        drive(op, Some(NotificationRecorder::new(self.cx.clone()))).await
    }

    /// Run a user-initiated mutation with failure recording: on error the
    /// recording layer records one `command-error` notification. The orchestrator
    /// owns all notification recording — the renderer only displays what the
    /// notification channel pushes.
    pub async fn execute_recorded<C: Command<Ctx>>(&self, c: C) -> Result<()> {
        let cx = self.cx.clone();
        let op = Op {
            action: std::any::type_name::<C>(),
            kind: OpKind::Command,
            notable: None,
            fut: Box::pin(async move { c.handle(&cx).await }),
        };
        drive(op, Some(NotificationRecorder::new(self.cx.clone()))).await
    }
}

/// Whether an envelope carries a mutation or a read. Layers branch their span
/// name on it; it never reaches a handler.
#[derive(Clone, Copy)]
pub(crate) enum OpKind {
    Command,
    Query,
}

/// An erased dispatch envelope: the operation's identity (`action`, `kind`) plus
/// its handler invocation as a pre-built `'static` future. The future owns its
/// `Cx` clone and the message, so the typed `&Cx` borrow is resolved before the
/// envelope is built and the pipeline can drive a uniform `Op<T>` for any `T`.
pub(crate) struct Op<T> {
    pub(crate) action: &'static str,
    pub(crate) kind: OpKind,
    /// The notification to record after a notification-worthy signal's handler
    /// runs. `None` for ordinary commands and queries -- the recording layer
    /// passes them through untouched.
    pub(crate) notable: Option<RecordNotification>,
    pub(crate) fut: BoxFuture<T>,
}

/// Drive one envelope through the middleware stack. The stack's set and order
/// live in `crate::middleware::pipeline`; the recording layer's dependency (the
/// notification store, via `Ctx`) is supplied here, so it is a pass-through when
/// `recorder` is `None` (the ordinary command/query path).
async fn drive<T: Send + 'static>(op: Op<T>, recorder: Option<NotificationRecorder>) -> Result<T> {
    use tower::ServiceExt;
    middleware::pipeline(recorder).oneshot(op).await
}

/// The innermost `Service`: it just drives the envelope's pre-built future.
/// Uniform over the output type `T`.
#[derive(Clone, Copy)]
pub(crate) struct HandlerService;

impl<T> Service<Op<T>> for HandlerService {
    type Response = T;
    type Error = Error;
    type Future = BoxFuture<T>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, op: Op<T>) -> Self::Future {
        op.fut
    }
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

    trait Counter: Send + Sync {
        fn increment(&self);
    }

    #[test]
    fn broadcast_dispatch_calls_all_subscribers_in_subscribe_order() {
        let bc: Broadcast<dyn Counter> = Broadcast::default();
        let order: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));

        for id in 0u32..3 {
            let order = Arc::clone(&order);
            // Use a concrete struct that captures `id` and records it.
            struct Rec {
                id: u32,
                order: Arc<Mutex<Vec<u32>>>,
            }
            impl Counter for Rec {
                fn increment(&self) {
                    self.order.lock().unwrap().push(self.id);
                }
            }
            bc.subscribe(Arc::new(Rec {
                id,
                order: Arc::clone(&order),
            }));
        }

        bc.dispatch(|s| s.increment());

        assert_eq!(*order.lock().unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn broadcast_dispatch_with_no_subscribers_is_a_noop() {
        let bc: Broadcast<dyn Counter> = Broadcast::default();
        // Must not panic and must not call anything (nothing to call).
        bc.dispatch(|s| s.increment());
    }

    /// A sink that records its own label whenever it is invoked.
    struct Tag(&'static str, Arc<Mutex<Vec<&'static str>>>);
    impl Counter for Tag {
        fn increment(&self) {
            self.1.lock().unwrap().push(self.0);
        }
    }

    #[test]
    fn registry_dispatch_reaches_only_the_keys_sinks() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Registry<dyn Counter> = Registry::default();

        reg.register("A", Arc::new(Tag("a", Arc::clone(&log))));
        reg.register("B", Arc::new(Tag("b", Arc::clone(&log))));

        reg.dispatch("A", |s| s.increment());

        assert_eq!(*log.lock().unwrap(), ["a"]);
    }

    #[test]
    fn registry_removed_sink_receives_no_further_events() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Registry<dyn Counter> = Registry::default();

        let id = reg.register("A", Arc::new(Tag("a", Arc::clone(&log))));
        reg.remove("A", id);

        reg.dispatch("A", |s| s.increment());

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn registry_removing_one_sink_leaves_others_under_the_same_key() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Registry<dyn Counter> = Registry::default();

        let id = reg.register("A", Arc::new(Tag("first", Arc::clone(&log))));
        reg.register("A", Arc::new(Tag("second", Arc::clone(&log))));
        reg.remove("A", id);

        reg.dispatch("A", |s| s.increment());

        assert_eq!(*log.lock().unwrap(), ["second"]);
    }

    #[test]
    fn registry_remove_key_drops_every_sink_under_that_key() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Registry<dyn Counter> = Registry::default();

        reg.register("A", Arc::new(Tag("first", Arc::clone(&log))));
        reg.register("A", Arc::new(Tag("second", Arc::clone(&log))));
        reg.remove_key("A");

        reg.dispatch("A", |s| s.increment());

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn registry_remove_key_leaves_other_keys_intact() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Registry<dyn Counter> = Registry::default();

        reg.register("A", Arc::new(Tag("a", Arc::clone(&log))));
        reg.register("B", Arc::new(Tag("b", Arc::clone(&log))));
        reg.remove_key("A");

        reg.dispatch("A", |s| s.increment());
        reg.dispatch("B", |s| s.increment());

        assert_eq!(*log.lock().unwrap(), ["b"]);
    }

    #[test]
    fn registry_register_and_remove_are_safe_during_an_in_flight_dispatch() {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::default();
        let reg: Arc<Registry<dyn Counter>> = Arc::default();

        reg.register("A", Arc::new(Tag("a", Arc::clone(&log))));

        // Hammer register/remove from a second thread while the main thread
        // dispatches in a tight loop. Neither must panic.
        let mutator = {
            let reg = Arc::clone(&reg);
            let log = Arc::clone(&log);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    let id = reg.register("B", Arc::new(Tag("b", Arc::clone(&log))));
                    reg.remove("B", id);
                }
            })
        };
        for _ in 0..1_000 {
            reg.dispatch("A", |s| s.increment());
        }
        mutator.join().unwrap();

        // A registration made after the race takes effect on the next dispatch.
        log.lock().unwrap().clear();
        reg.register("B", Arc::new(Tag("b", Arc::clone(&log))));
        reg.dispatch("B", |s| s.increment());
        assert_eq!(*log.lock().unwrap(), ["b"]);
    }

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
    impl Command<Arc<Mutex<u32>>> for Increment {
        async fn handle(&self, cx: &Arc<Mutex<u32>>) -> Result<()> {
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
        let bus = Bus::new(Arc::new(Mutex::new(0)));
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

    type Trace = Arc<Mutex<Vec<&'static str>>>;

    /// A layer that pushes its marker into a shared trace when its inner service
    /// is called, then delegates unchanged -- the test instrument that proves a
    /// layer observes a dispatch without altering it.
    struct MarkerLayer(&'static str, Trace);

    impl<S> tower::Layer<S> for MarkerLayer {
        type Service = Marker<S>;
        fn layer(&self, inner: S) -> Self::Service {
            Marker {
                tag: self.0,
                trace: Arc::clone(&self.1),
                inner,
            }
        }
    }

    struct Marker<S> {
        tag: &'static str,
        trace: Trace,
        inner: S,
    }

    impl<S, T> tower::Service<Op<T>> for Marker<S>
    where
        S: tower::Service<Op<T>, Response = T, Error = Error>,
    {
        type Response = T;
        type Error = Error;
        type Future = S::Future;

        fn poll_ready(
            &mut self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::result::Result<(), Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, op: Op<T>) -> Self::Future {
            self.trace.lock().unwrap().push(self.tag);
            self.inner.call(op)
        }
    }

    fn op<T: Send + 'static>(value: T) -> Op<T> {
        Op {
            action: "test.op",
            kind: OpKind::Command,
            notable: None,
            fut: Box::pin(async move { Ok(value) }),
        }
    }

    #[tokio::test]
    async fn an_installed_layer_observes_a_dispatch_without_changing_the_result() {
        use tower::{ServiceBuilder, ServiceExt};

        let trace: Trace = Arc::default();
        let service = ServiceBuilder::new()
            .layer(MarkerLayer("observed", Arc::clone(&trace)))
            .service(HandlerService);

        let out = service.oneshot(op(7u32)).await.unwrap();

        assert_eq!(out, 7);
        assert_eq!(*trace.lock().unwrap(), ["observed"]);
    }

    #[tokio::test]
    async fn two_layers_run_in_composition_order_around_the_handler() {
        use tower::{ServiceBuilder, ServiceExt};

        let trace: Trace = Arc::default();
        let service = ServiceBuilder::new()
            .layer(MarkerLayer("outer", Arc::clone(&trace)))
            .layer(MarkerLayer("inner", Arc::clone(&trace)))
            .service(HandlerService);

        service.oneshot(op(())).await.unwrap();

        assert_eq!(*trace.lock().unwrap(), ["outer", "inner"]);
    }

    /// 3.1: a `Notable` op dispatched through the bus is observed by the
    /// recording layer, which reads its `notification()`. The notifications-
    /// changed sink captures what the layer read, proving the observation.
    #[tokio::test]
    async fn a_notable_op_is_observed_by_the_recording_layer() {
        use crate::app::notification::SurfaceStarted;

        let ctx = crate::boot::test_ctx().await.unwrap();
        let observed: Arc<Mutex<Vec<String>>> = Arc::default();
        let sink = Arc::clone(&observed);
        ctx.notifications_changed()
            .subscribe(Arc::new(move |n: &RecordNotification| {
                sink.lock().unwrap().push(n.category.clone());
            }));

        Bus::new(ctx)
            .execute_notable(SurfaceStarted {
                surface_id: "sf_1".to_owned(),
                session_id: "se_1".to_owned(),
                ts: 7,
            })
            .await
            .unwrap();

        assert_eq!(
            *observed.lock().unwrap(),
            vec!["surface-started".to_owned()]
        );
    }

    /// 3.3: one `Notable` signal records exactly one notification, and the single
    /// recording point (the layer) announces it exactly once -- no second
    /// recorder records the same signal.
    #[tokio::test]
    async fn one_notable_signal_records_exactly_one_notification() {
        use crate::app::notification::{ListNotifications, SurfaceStarted};

        let ctx = crate::boot::test_ctx().await.unwrap();
        let announced: Arc<Mutex<u32>> = Arc::default();
        let counter = Arc::clone(&announced);
        ctx.notifications_changed()
            .subscribe(Arc::new(move |_n: &RecordNotification| {
                *counter.lock().unwrap() += 1;
            }));

        let bus = Bus::new(ctx);
        bus.execute_notable(SurfaceStarted {
            surface_id: "sf_1".to_owned(),
            session_id: "se_1".to_owned(),
            ts: 7,
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListNotifications {
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(*announced.lock().unwrap(), 1);
    }

    /// An ordinary command carries no notable payload and records nothing -- the
    /// recording layer is a pass-through for the 115 existing commands.
    #[tokio::test]
    async fn an_ordinary_command_records_no_notification() {
        use crate::app::notification::ListNotifications;

        let ctx = crate::boot::test_ctx().await.unwrap();

        struct NoopOverCtx;
        impl Command<Ctx> for NoopOverCtx {
            async fn handle(&self, _cx: &Ctx) -> Result<()> {
                Ok(())
            }
        }

        let bus = Bus::new(ctx);
        bus.execute(NoopOverCtx).await.unwrap();

        let listing = bus
            .query(ListNotifications {
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(listing.items.is_empty());
    }
}
