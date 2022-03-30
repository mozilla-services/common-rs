use gethostname::gethostname;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, io::Write};
use tracing::{Event, Level, Subscriber};
use tracing_bunyan_formatter::JsonStorage;
use tracing_subscriber::{fmt::MakeWriter, layer::Context};

const MOZLOG_VERSION: &str = "2.0";

/// This layer is exclusively concerned with formatting information using the
/// [MozLog format](https://wiki.mozilla.org/Firefox/Services/Logging). It relies
/// on the upstream [`crate::JsonStorageLayer`] to get access
/// to the fields attached to each span.
///
/// # Example
///
/// ```
/// use tracing_actix_web_mozlog::{JsonStorageLayer, MozLogFormatLayer};
/// use tracing_subscriber::layer::SubscriberExt;
/// let subscriber = tracing_subscriber::registry()
///     .with(JsonStorageLayer)
///     .with(MozLogFormatLayer::new("service-name", std::io::stdout));
/// ```
pub struct MozLogFormatLayer<W: for<'a> MakeWriter<'a> + 'static> {
    name: String,
    pid: u32,
    hostname: String,
    make_writer: W,
}

/// A logging message in MozLog format, adapted to Tracing.
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MozLogMessage {
    /// Number of nanoseconds since the UNIX epoch (which is UTC)
    pub timestamp: i128,

    /// Type of message i.e. "request.summary"
    #[serde(rename = "type")]
    pub message_type: String,

    /// Data source, server that is doing the logging, e.g. “Sync-1_5”
    pub logger: String,

    /// Hostname that generated the message
    pub hostname: String,

    /// Envelope version; log format version
    pub env_version: String,

    /// Process ID that generated the message
    pub pid: u32,

    /// Syslog severity levels
    pub severity: u32,

    /// Hash of fields
    pub fields: HashMap<String, Value>,
}

impl<W: for<'a> MakeWriter<'a> + 'static> MozLogFormatLayer<W> {
    /// Create a new moz log subscriber.
    pub fn new<S: AsRef<str>>(name: S, make_writer: W) -> Self {
        Self {
            name: name.as_ref().to_string(),
            make_writer,
            pid: std::process::id(),
            hostname: gethostname().to_string_lossy().into_owned(),
        }
    }

    fn emit(&self, mut buffer: Vec<u8>) -> Result<(), std::io::Error> {
        buffer.write_all(b"\n")?;
        self.make_writer.make_writer().write_all(&buffer)
    }
}

impl<S, W> tracing_subscriber::Layer<S> for MozLogFormatLayer<W>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    W: for<'a> MakeWriter<'a> + 'static,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Use a closure that returns a `Result` to enable usage of the `?`
        // operator and make clearer code. This is called immediately below.
        let make_log_line = || {
            let mut event_visitor = JsonStorage::default();
            event.record(&mut event_visitor);

            let mut values: HashMap<String, Value> = event_visitor
                .values()
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();

            let spans = {
                let mut span_names = vec![];
                let mut current = ctx.lookup_current();
                while let Some(span) = &current {
                    {
                        let ext = span.extensions();
                        let span_visitor = ext
                            .get::<JsonStorage>()
                            .expect("MozLogFormatLayer requires JsonStorage layer");
                        for (k, v) in span_visitor.values() {
                            values.entry(k.to_string()).or_insert_with(|| v.clone());
                        }
                    }

                    span_names.push(span.name());
                    current = span.parent();
                }
                span_names.reverse();
                span_names.join(",")
            };

            // See https://en.wikipedia.org/wiki/Syslog#Severity_levels
            let severity = match *event.metadata().level() {
                Level::ERROR => 3, // Syslog Error
                Level::WARN => 4,  // Syslog Warning
                Level::INFO => 5,  // Syslog Normal
                Level::DEBUG => 6, // Syslog Informational
                Level::TRACE => 7, // Syslog Debug
            };

            let type_field = values.remove("type");
            let raw_type_field = values.remove("r#type");
            values.insert("spans".to_string(), spans.into());

            let v = MozLogMessage {
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                message_type: type_field
                    .or(raw_type_field)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<unknown>".to_string()),
                logger: self.name.clone(),
                hostname: self.hostname.clone(),
                env_version: MOZLOG_VERSION.to_string(),
                pid: self.pid,
                severity,
                fields: values,
            };

            // If there is an error, just squash it quietly. After all, if we
            // failed to log, we can't exactly log an error.
            serde_json::to_vec(&v).map_err(|_| ())
        };

        let log_line_result: Result<Vec<u8>, ()> = make_log_line();
        // Discard any errors, since they probably can't be logged anyways.
        if let Ok(log_line) = log_line_result {
            let _ = self.emit(log_line);
        }
    }
}
