//! Surface event contract: borrowed-enum event + sink trait + fan-out impl.
//!
//! `SurfaceEvent<'a>` carries all output from a surface runtime to its
//! subscribers. Payloads are plain built-in Rust types borrowed from the decoded
//! frame, so no copy occurs on the dispatch path. Subscribers choose to borrow,
//! copy, or clone at their own boundary.
//!
//! `SurfaceSink` is the composition point (D2): middleware wraps it, the
//! `Broadcast<dyn SurfaceSink>` fan-out impl and the closure blanket impl (D8)
//! are the two terminal implementations shipped here.

use crate::shared::bus::Broadcast;

/// Every output variant a surface runtime can produce. All payloads are plain
/// built-ins borrowed from the decoded frame -- zero extra copy through fan-out.
pub enum SurfaceEvent<'a> {
    /// Raw PTY / process bytes to render.
    Bytes(&'a [u8]),
    /// Human-readable lifecycle status transition (e.g. `"live"`, `"pending"`).
    Status(&'a str),
    /// The surface process has exited; qualifier carries the reason or code.
    Exit(&'a str),
    /// A non-recoverable surface-level error after open.
    Error(&'a str),
}

/// Receives surface events addressed by primitive surface id.
///
/// Implementations must be `Send + Sync + 'static` so they can be held behind
/// `Arc<dyn SurfaceSink>` and shared across async tasks. The method is
/// synchronous: implementations must not block.
pub trait SurfaceSink: Send + Sync + 'static {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>);
}

/// `Broadcast<dyn SurfaceSink>` is itself a `SurfaceSink`: calling `emit`
/// dispatches to every registered subscriber synchronously.
impl SurfaceSink for Broadcast<dyn SurfaceSink> {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        self.dispatch(|s| s.emit(surface, event));
    }
}

/// Any `Fn(&str, &SurfaceEvent<'_>) + Send + Sync + 'static` is a sink, so
/// callers can subscribe with a closure and need no explicit struct or impl.
impl<F> SurfaceSink for F
where
    F: Fn(&str, &SurfaceEvent<'_>) + Send + Sync + 'static,
{
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        self(surface, event)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A recording sink that appends `(surface_id, tag)` pairs.
    struct Recorder(Arc<Mutex<Vec<(String, &'static str)>>>);

    impl SurfaceSink for Recorder {
        fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
            let tag = match event {
                SurfaceEvent::Bytes(_) => "bytes",
                SurfaceEvent::Status(_) => "status",
                SurfaceEvent::Exit(_) => "exit",
                SurfaceEvent::Error(_) => "error",
            };
            self.0.lock().unwrap().push((surface.to_owned(), tag));
        }
    }

    #[test]
    fn one_borrowed_event_reaches_all_subscribers_without_copy() {
        let log: Arc<Mutex<Vec<(String, &'static str)>>> = Arc::default();
        let bc: Arc<Broadcast<dyn SurfaceSink>> = Arc::default();

        for _ in 0..3 {
            bc.subscribe(Arc::new(Recorder(Arc::clone(&log))));
        }

        let payload: &[u8] = b"hello";
        bc.emit("surf_1", &SurfaceEvent::Bytes(payload));

        let got = log.lock().unwrap();
        assert_eq!(got.len(), 3);
        assert!(got
            .iter()
            .all(|(id, tag)| id == "surf_1" && *tag == "bytes"));
    }

    #[test]
    fn closure_blanket_impl_receives_the_event() {
        let log: Arc<Mutex<Vec<String>>> = Arc::default();
        let bc: Arc<Broadcast<dyn SurfaceSink>> = Arc::default();

        let captured = Arc::clone(&log);
        bc.subscribe(Arc::new(move |surface: &str, _event: &SurfaceEvent<'_>| {
            captured.lock().unwrap().push(surface.to_owned());
        }));

        bc.emit("surf_2", &SurfaceEvent::Status("live"));

        assert_eq!(log.lock().unwrap().as_slice(), ["surf_2"]);
    }
}
