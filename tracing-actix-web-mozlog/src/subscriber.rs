use gethostname::gethostname;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, io::Write, string::ToString};
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
pub struct MozLogFormatLayer<W: MakeWriter + 'static> {
    name: String,
    pid: u32,
    hostname: String,
    make_writer: W,
    type_required_for_level: Option<Level>,
    unknown_type_handler: Option<Box<dyn Send + Sync + Fn(&Event<'_>) -> Option<String>>>,
}

/// A logging message in MozLog format, adapted to Tracing.
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MozLogMessage {
    /// Number of nanoseconds since the UNIX epoch (which is UTC)
    pub timestamp: i64,

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

impl<W: MakeWriter + 'static> MozLogFormatLayer<W> {
    /// Create a new moz log subscriber.
    pub fn new<S: AsRef<str>>(name: S, make_writer: W) -> Self {
        Self {
            name: name.as_ref().to_string(),
            make_writer,
            pid: std::process::id(),
            hostname: gethostname().to_string_lossy().into_owned(),
            type_required_for_level: None,
            unknown_type_handler: None,
        }
    }

    /// If set to Some, any log line that is at this level or a less verbose one
    /// must have a type set. Any message that violates this rule will cause an
    /// extra error message to be emitted.
    pub fn with_type_required_for_level(mut self, type_required_for_level: Option<Level>) -> Self {
        self.type_required_for_level = type_required_for_level;
        self
    }

    /// If set to Some, any event that has an unknown type field will be passed
    /// to this closure. If the closure returns a Some(String), that value will
    /// be used for the type.
    pub fn with_unknown_type_handler(
        mut self,
        unknown_type_handler: Option<Box<dyn Send + Sync + Fn(&Event<'_>) -> Option<String>>>,
    ) -> Self {
        self.unknown_type_handler = unknown_type_handler;
        self
    }

    fn emit(&self, mut buffer: Vec<u8>) -> Result<(), std::io::Error> {
        buffer.write_all(b"\n")?;
        self.make_writer.make_writer().write_all(&buffer)
    }

    #[must_use]
    fn handle_missing_type(
        &self,
        event: &Event<'_>,
        values: &HashMap<String, serde_json::Value>,
    ) -> Option<Result<Vec<u8>, ()>> {
        let event_level = *event.metadata().level();
        if let Some(type_required_for_level) = self.type_required_for_level {
            if event_level <= type_required_for_level {
                let error_message = MozLogMessage {
                    timestamp: chrono::Utc::now().timestamp_nanos(),
                    message_type: "mozlog.missing-type".to_string(),
                    logger: self.name.clone(),
                    hostname: self.hostname.clone(),
                    env_version: MOZLOG_VERSION.to_string(),
                    pid: self.pid,
                    severity: 3, // error
                    fields: hashmap! {
                        "message".to_string() => format!("events with level {} require a type to be set", event_level).into(),
                        "original_level".to_string() => event_level.to_string().into(),
                        "original_message".to_string() => values.get("message")
                            .map_or_else(|| serde_json::Value::String("<none>".to_string()), Clone::clone),
                        "spans".to_string() => values.get("spans").expect("was inserted earlier").clone(),
                    },
                };

                // If there is an error, just squash it quietly. After all, if we
                // failed to log, we can't exactly log an error.
                return Some(serde_json::to_vec(&error_message).map_err(|_| ()));
            }
        }
        None
    }
}

impl<S, W> tracing_subscriber::Layer<S> for MozLogFormatLayer<W>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    W: MakeWriter + 'static,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut event_visitor = JsonStorage::default();
        event.record(&mut event_visitor);

        let mut values: HashMap<String, Value> = event_visitor
            .values()
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
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
                        values.entry((*k).to_string()).or_insert_with(|| v.clone());
                    }
                }

                span_names.push(span.name());
                current = span.parent();
            }
            span_names.reverse();
            span_names.join(",").into()
        };
        values.insert("spans".to_string(), spans);

        // See https://en.wikipedia.org/wiki/Syslog#Severity_levels
        let event_level = *event.metadata().level();
        let severity = match event_level {
            Level::ERROR => 3, // Syslog Error
            Level::WARN => 4,  // Syslog Warning
            Level::INFO => 5,  // Syslog Normal
            Level::DEBUG => 6, // Syslog Informational
            Level::TRACE => 7, // Syslog Debug
        };

        let mut log_lines: Vec<Result<Vec<u8>, _>> = Vec::with_capacity(2);

        let message_type = {
            let type_field = values.remove("type");
            let raw_type_field = values.remove("r#type");
            let combined = type_field
                .or(raw_type_field)
                .and_then(|v| v.as_str().map(ToString::to_string))
                .or_else(|| {
                    self.unknown_type_handler
                        .as_ref()
                        .and_then(|handler| handler(event))
                });

            combined.unwrap_or_else(|| {
                log_lines.extend(self.handle_missing_type(event, &values).into_iter());
                "<unknown>".to_string()
            })
        };

        let v = MozLogMessage {
            timestamp: chrono::Utc::now().timestamp_nanos(),
            message_type,
            logger: self.name.clone(),
            hostname: self.hostname.clone(),
            env_version: MOZLOG_VERSION.to_string(),
            pid: self.pid,
            severity,
            fields: values,
        };

        // If there is an error, just squash it quietly. After all, if we
        // failed to log, we can't exactly log an error.
        log_lines.push(serde_json::to_vec(&v).map_err(|_| ()));

        // Discard any errors, since they probably can't be logged anyways.
        for log_line in log_lines.into_iter().flatten() {
            self.emit(log_line).ok();
        }
    }
}
