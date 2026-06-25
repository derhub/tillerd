//! Log-follow event contract: a borrowed event + sink trait + fan-out impl.
//!
//! `LogLine<'a>` carries one freshly-appended log line, borrowed from the read
//! buffer so no copy occurs on the dispatch path. Subscribers choose to borrow,
//! copy, or clone at their own boundary.
//!
//! `LogSink` is the composition point: the `Registry<dyn LogSink>` key-scoped
//! fan-out impl and the closure blanket impl are the two terminal implementations
//! shipped here.

use crate::shared::bus::Registry;

/// One appended line from a followed `.log` file, borrowed from the read buffer
/// -- zero extra copy through fan-out. Newline stripped.
pub struct LogLine<'a>(pub &'a str);

/// Receives appended log lines addressed by service key (the file-name prefix).
///
/// Implementations must be `Send + Sync + 'static` so they can be held behind
/// `Arc<dyn LogSink>` and shared across async tasks. The method is synchronous:
/// implementations must not block.
pub trait LogSink: Send + Sync + 'static {
    fn emit(&self, service: &str, line: &LogLine<'_>);
}

/// `Registry<dyn LogSink>` dispatches a borrowed line to every sink registered
/// under the line's service key.
impl LogSink for Registry<dyn LogSink> {
    fn emit(&self, service: &str, line: &LogLine<'_>) {
        self.dispatch(service, |s| s.emit(service, line));
    }
}

/// Any `Fn(&str, &LogLine<'_>) + Send + Sync + 'static` is a sink, so callers can
/// subscribe with a closure and need no explicit struct or impl.
impl<F> LogSink for F
where
    F: Fn(&str, &LogLine<'_>) + Send + Sync + 'static,
{
    fn emit(&self, service: &str, line: &LogLine<'_>) {
        self(service, line)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A recording sink that appends `(service, line)` pairs.
    struct Recorder(Arc<Mutex<Vec<(String, String)>>>);

    impl LogSink for Recorder {
        fn emit(&self, service: &str, line: &LogLine<'_>) {
            self.0.lock().unwrap().push((service.to_owned(), line.0.to_owned()));
        }
    }

    #[test]
    fn a_borrowed_line_reaches_the_keys_sinks() {
        let log: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
        let reg: Registry<dyn LogSink> = Registry::default();

        reg.register("daemon", Arc::new(Recorder(Arc::clone(&log))));

        reg.emit("daemon", &LogLine("hello"));

        assert_eq!(
            log.lock().unwrap().as_slice(),
            [("daemon".to_owned(), "hello".to_owned())]
        );
    }

    #[test]
    fn a_line_for_another_service_does_not_reach_the_sink() {
        let log: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
        let reg: Registry<dyn LogSink> = Registry::default();

        reg.register("daemon", Arc::new(Recorder(Arc::clone(&log))));

        reg.emit("gate", &LogLine("other"));

        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn closure_blanket_impl_receives_the_line() {
        let log: Arc<Mutex<Vec<String>>> = Arc::default();
        let reg: Registry<dyn LogSink> = Registry::default();

        let captured = Arc::clone(&log);
        reg.register(
            "daemon",
            Arc::new(move |_service: &str, line: &LogLine<'_>| {
                captured.lock().unwrap().push(line.0.to_owned());
            }),
        );

        reg.emit("daemon", &LogLine("via-closure"));

        assert_eq!(log.lock().unwrap().as_slice(), ["via-closure"]);
    }
}
