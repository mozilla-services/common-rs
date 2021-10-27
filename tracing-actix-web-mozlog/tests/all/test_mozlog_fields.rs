use crate::utils::{log_test, LogWatcher};
use maplit::hashmap;
use pretty_assertions::assert_eq;
use serde_json::json;
use tracing::{event, span, Level};
use tracing_actix_web_mozlog::MozLogMessage;

#[test]
fn test_format() {
    let mut log_watcher: LogWatcher<MozLogMessage> = log_test(None, || {
        event!(
            Level::INFO,
            r#type = "test",
            "simple event without a parent span"
        );
    });

    let events = log_watcher.events();

    assert!(!events.is_empty(), "There should be at least one event");
    assert_eq!(
        events,
        &vec![MozLogMessage {
            message_type: "test".to_string(),
            logger: "test-logger".to_string(),
            env_version: "2.0".to_string(),
            severity: 5,
            fields: hashmap!(
                "message".to_string() => "simple event without a parent span".into(),
                "spans".to_string() => "".into(),
            ),
            ..events[0].clone()
        }]
    );

    assert!(events[0].pid > 0, "Should have a positive PID");
    assert_ne!(events[0].hostname, "", "Should have a non-empty hostname");

    // Trying to assert that the timestamp is exactly right is going to be
    // difficult. Instead this tests that it is the right order of magnitude. To
    // do this we interpret it as nanoseconds 1x10-9, as given in the spec, and
    // then check that it is a time that occurs roughly sometime this century.
    // If the given number was in milliseconds, seconds, or any other order of
    // magnitude, the below would fail. Gigaseconds are 1x10^9 seconds. 1
    // gigaseconds since epoch is sometime in the year 2001, and 4 gigaseconds
    // is in 2096.
    let gigaseconds = events[0].timestamp / i64::pow(10, 18);
    assert!(
        (1..=4).contains(&gigaseconds),
        "Should have a timestamp in this century"
    );
}

#[test]
fn test_log_level_to_severity() {
    let mut log_watcher: LogWatcher<MozLogMessage> = log_test(None, || {
        event!(Level::ERROR, "error");
        event!(Level::WARN, "warn");
        event!(Level::INFO, "info");
        event!(Level::DEBUG, "debug");
        event!(Level::TRACE, "trace");
    });

    log_watcher.has(|msg| {
        msg.fields.get("msg") == Some(&json!("error"))
            && msg.fields.get("severity") == Some(&json!(3))
    });
    log_watcher.has(|msg| {
        msg.fields.get("msg") == Some(&json!("warn"))
            && msg.fields.get("severity") == Some(&json!(4))
    });
    log_watcher.has(|msg| {
        msg.fields.get("msg") == Some(&json!("info"))
            && msg.fields.get("severity") == Some(&json!(5))
    });
    log_watcher.has(|msg| {
        msg.fields.get("msg") == Some(&json!("debug"))
            && msg.fields.get("severity") == Some(&json!(6))
    });
    log_watcher.has(|msg| {
        msg.fields.get("msg") == Some(&json!("trace"))
            && msg.fields.get("severity") == Some(&json!(7))
    });
}

#[test]
fn test_span_is_listed() {
    let mut log_watcher: LogWatcher = log_test(None, || {
        let span = span!(Level::INFO, "test_span");
        let _guard = span.enter();
        event!(Level::INFO, "test_event");
    });

    let events = log_watcher.events();
    assert!(!events.is_empty());

    assert_eq!(
        events,
        &vec![MozLogMessage {
            fields: hashmap!(
                "message".to_string() => "test_event".into(),
                "spans".to_string() => "test_span".into(),
            ),
            ..events[0].clone()
        }]
    );
}

#[test]
fn test_nested_spans() {
    let mut log_watcher: LogWatcher = log_test(None, || {
        event!(Level::INFO, "event at nesting 0");
        let _guard1 = span!(Level::INFO, "test_span_1").entered();
        event!(Level::INFO, "event at nesting 1");
        let _guard2 = span!(Level::INFO, "test_span_2").entered();
        event!(Level::INFO, "event at nesting 2");
    });

    let events = log_watcher.events();
    assert!(!events.is_empty());

    assert_eq!(
        events,
        &vec![
            MozLogMessage {
                fields: hashmap!(
                    "message".to_string() => "event at nesting 0".into(),
                    "spans".to_string() => "".into(),
                ),
                ..events[0].clone()
            },
            MozLogMessage {
                fields: hashmap!(
                    "message".to_string() => "event at nesting 1".into(),
                    "spans".to_string() => "test_span_1".into(),
                ),
                ..events[1].clone()
            },
            MozLogMessage {
                fields: hashmap!(
                    "message".to_string() => "event at nesting 2".into(),
                    "spans".to_string() => "test_span_1,test_span_2".into(),
                ),
                ..events[2].clone()
            }
        ]
    );
}

#[test]
fn events_inherit_fields() {
    let mut log_watcher: LogWatcher = log_test(None, || {
        let _guard = span!(Level::INFO, "test_span", color = "red").entered();
        event!(Level::INFO, "test_event");
    });
    let events = log_watcher.events();
    assert!(!events.is_empty());

    assert_eq!(
        events,
        &vec![MozLogMessage {
            fields: hashmap!(
                "message".to_string() => "test_event".into(),
                "spans".to_string() => "test_span".into(),
                "color".to_string() => "red".into(),
            ),
            ..events[0].clone()
        }]
    );
}

#[test]
fn innermost_value_wins() {
    let mut log_watcher: LogWatcher = log_test(None, || {
        let _outer = span!(Level::INFO, "outer", a = 1, b = 1, c = 1).entered();
        let _inner = span!(Level::INFO, "inner", b = 2, c = 2).entered();
        event!(Level::INFO, c = 3, "test_event");
    });
    let events = log_watcher.events();
    assert!(!events.is_empty());

    assert_eq!(
        events,
        &vec![MozLogMessage {
            fields: hashmap!(
                "message".to_string() => "test_event".into(),
                "spans".to_string() => "outer,inner".into(),
                "a".to_string() => 1.into(),
                "b".to_string() => 2.into(),
                "c".to_string() => 3.into(),
            ),
            ..events[0].clone()
        }]
    );
}
