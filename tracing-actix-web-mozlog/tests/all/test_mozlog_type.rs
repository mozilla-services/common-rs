use crate::utils::{log_test, LogWatcher};
use maplit::hashmap;
use pretty_assertions::assert_eq;
use tracing::{event, Level};
use tracing_actix_web_mozlog::MozLogMessage;

#[test]
fn test_type_can_be_required() {
    let mut log_watcher: LogWatcher<MozLogMessage> = log_test(Some(Level::INFO), None, || {
        event!(Level::INFO, "no type");
    });

    let events = log_watcher.events();

    assert_eq!(
        events.len(),
        2,
        "There should be the emitted event and an extra one for the error"
    );
    assert_eq!(
        events,
        &vec![
            MozLogMessage {
                message_type: "mozlog.missing-type".to_string(),
                severity: 3, // 3 = ERROR
                fields: hashmap!(
                    "message".to_string() => "events with level INFO require a type to be set".into(),
                    "original_message".to_string() => "no type".into(),
                    "spans".to_string() => "".into(),
                    "original_level".to_string() => "INFO".into(),
                ),
                ..events[0].clone()
            },
            MozLogMessage {
                fields: hashmap!(
                    "message".to_string() => "no type".into(),
                    "spans".to_string() => "".into(),
                ),
                ..events[1].clone()
            }
        ]
    );
}

#[test]
fn test_unknown_type_can_be_filled_in() {
    let mut log_watcher: LogWatcher<MozLogMessage> = log_test(
        None,
        Some(Box::new(|_ev| Some("fallback-type".to_string()))),
        || {
            event!(Level::INFO, "no type");
        },
    );

    let events = log_watcher.events();
    assert!(!events.is_empty());
    assert_eq!(
        events,
        &vec![MozLogMessage {
            message_type: "fallback-type".to_string(),
            fields: hashmap!(
                "message".to_string() => "no type".into(),
                "spans".to_string() => "".into(),
            ),
            ..events[0].clone()
        }]
    );
}

#[test]
fn test_fallback_types_dont_trigger_an_error() {
    let mut log_watcher: LogWatcher<MozLogMessage> = log_test(
        Some(Level::INFO),
        Some(Box::new(|_ev| Some("fallback-type".to_string()))),
        || {
            event!(Level::INFO, "no type");
        },
    );

    let events = log_watcher.events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        MozLogMessage {
            message_type: "fallback-type".to_string(),
            severity: 5, // INFO
            ..events[0].clone()
        }
    );
}
