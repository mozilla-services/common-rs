//! Testing utils

use serde::de::DeserializeOwned;
use std::{
    future::Future,
    io::Write,
    sync::{Arc, Mutex},
};
use tracing::{Level, Subscriber};
use tracing_actix_web_mozlog::{JsonStorageLayer, MozLogFormatLayer, MozLogMessage};
use tracing_futures::WithSubscriber;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, Registry};

/// Run a closure in an environment configured to use [`MozLogLayer`], and return
/// a log watcher that cna make assertions about the tracing logs that occurred
/// while running the closure.
pub fn log_test<E, F>(type_required_for: Option<Level>, test_inner: F) -> LogWatcher<E>
where
    E: 'static + DeserializeOwned + Default,
    F: FnOnce(),
{
    let (log_watcher, subscriber) = make_test_subscriber(type_required_for);
    tracing::subscriber::with_default(subscriber, test_inner);
    log_watcher
}

/// A version of [`log_test`] that can handle async inner tests.
pub async fn log_test_async<E, F, Fut>(
    type_required_for: Option<Level>,
    test_inner: F,
) -> LogWatcher<E>
where
    E: 'static + DeserializeOwned + Default,
    F: FnOnce() -> Fut,
    Fut: Future,
{
    let (log_watcher, subscriber) = make_test_subscriber(type_required_for);
    test_inner().with_subscriber(subscriber).await;
    log_watcher
}

fn make_test_subscriber<E: Default>(
    type_required_for: Option<Level>,
) -> (LogWatcher<E>, impl Subscriber) {
    let log_watcher: LogWatcher<E> = LogWatcher::default();
    let log_watcher_writer = log_watcher.make_writer();
    let formatting_layer =
        MozLogFormatLayer::new("test-logger", move || log_watcher_writer.clone())
            .with_type_required_for_level(type_required_for);

    let subscriber = Registry::default()
        .with(JsonStorageLayer)
        .with(formatting_layer);

    (log_watcher, subscriber)
}

/// Helper to collect events emitted by Tracing and later make assertions about
/// the collected events.
///
/// The type parameter `E` is the message type that will be deserialized from the
/// bytes emitted by Tracing.
#[derive(Default)]
pub struct LogWatcher<E = MozLogMessage> {
    /// The raw bytes received from Tracing. Should represent new-line separated JSON objects.
    buf: Arc<Mutex<Vec<u8>>>,

    /// Events serialized from [`buf`](Self::buf). As valid JSON objects are
    /// parsed from `buf`, the corresponding bytes are removed from `buf`. This
    /// way if there are any partial writes, only the complete objects are
    /// processed from the buffer, leaving incomplete objects in place.
    events: Vec<E>,
}

impl<E> LogWatcher<E> {
    /// Make a new LogWatcher with some events pre-populated. For testing LogWatcher itself.
    #[allow(dead_code)]
    fn with_events(events: Vec<E>) -> Self {
        Self {
            events,
            buf: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<E> LogWatcher<E>
where
    E: DeserializeOwned,
    E: 'static,
{
    /// Test if any event this logger received matches `predicate`.
    ///
    /// # Example
    ///
    /// ```
    /// # use crate::utils::{LogWatcher, TracingJsonEvent};
    /// # use std::sync::{Arc, Mutex};
    /// # use tracing::Level;
    /// # let mut log_watcher = LogWatcher::with_events(vec![
    /// #     TracingJsonEvent {
    /// #         fields: maplit::hashmap!{ "message".to_string() => serde_json::json!("request success") },
    /// #         level: Level::INFO,
    /// #         target: String::new(),
    /// #         timestamp: String::new(),
    /// #     }
    /// # ]);
    /// #
    /// assert!(log_watcher.has(|msg| msg.field_contains("message", "request success")));
    /// ```
    pub fn has<F>(&mut self, predicate: F) -> bool
    where
        F: FnMut(&E) -> bool,
    {
        self.convert_events();
        self.events.iter().any(predicate)
    }

    pub fn events(&mut self) -> &Vec<E> {
        self.convert_events();
        &self.events
    }

    /// Iterate through `self.buf` to convert newline separated, completed J;SON
    /// objects into [`TracingJsonEvent`] instances that are placed in
    /// `self.events`.
    fn convert_events(&mut self) {
        let mut buf = self.buf.lock().expect("mutex was poisoned");
        let mut log_text = String::from_utf8(buf.clone()).expect("bad utf8");

        // Repeatedly find the next newline char...
        while let Some(idx) = log_text.find('\n') {
            // Split the string at that point...
            let mut message_json = log_text.split_off(idx);
            // and keep the left side, and return the right side to the string
            std::mem::swap(&mut message_json, &mut log_text);
            // Remove the leading newline that is left on the log line
            assert_eq!(log_text.chars().next(), Some('\n'));
            log_text.remove(0);

            // Skip blank lines
            if message_json.trim().is_empty() {
                continue;
            }

            // Now `message_join` contains the first line of logs, and `log_text` contains the rest.
            let message: E = serde_json::from_str(&message_json)
                .unwrap_or_else(|_| panic!("Bad JSON in log line: {}", &message_json));
            self.events.push(message);
        }

        // Now put the rest of the text back into the buffer.
        *buf = log_text.into_bytes();
        // and the mutex unlocks when it drops at the end of the function.
    }
}

impl<E> MakeWriter for LogWatcher<E> {
    type Writer = LogWatcherWriter;

    fn make_writer(&self) -> Self::Writer {
        LogWatcherWriter {
            buf: self.buf.clone(),
        }
    }
}

/// A helper that collects log events emitted from Tracing.
///
/// This is needed because Tracing consumes its subscribers. This type is a
/// "scout" that is split off from the main [`LogWatcher`] to give to Tracing,
/// and the data is written back to the parent type.
#[derive(Clone)]
pub struct LogWatcherWriter {
    /// The handle to the parent log watcher's buffer.
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for LogWatcherWriter {
    fn write(&mut self, new_bytes: &[u8]) -> std::io::Result<usize> {
        let mut buf = self
            .buf
            .lock()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        buf.extend(new_bytes.iter());
        Ok(new_bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
