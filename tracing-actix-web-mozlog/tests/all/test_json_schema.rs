use jsonschema::JSONSchema;
use lazy_static::lazy_static;
use serde_json::Value;
use tracing::{event, span, Level};

use crate::utils::{log_test, LogWatcher};

lazy_static! {
    static ref MOZLOG_SCHEMA: JSONSchema<'static> =
        JSONSchema::compile(&PARSED_SCHEMA).expect("schema is in invalid format");
    static ref PARSED_SCHEMA: Value =
        serde_json::from_str(include_str!("./mozlog_schema.json")).expect("schema json is invalid");
}

#[test]
fn logger_matches_schema() {
    let mut log_watcher: LogWatcher<Value> = log_test(None, || {
        event!(Level::INFO, "event at nesting 0");
        let _guard1 = span!(Level::INFO, "test_span_1").entered();
        event!(Level::INFO, "event at nesting 1");
        let _guard2 = span!(Level::INFO, "test_span_2").entered();
        event!(Level::INFO, "event at nesting 2");
    });

    for event in log_watcher.events() {
        let errors = match MOZLOG_SCHEMA.validate(event) {
            Ok(()) => None,
            Err(errors) => Some(errors.collect::<Vec<_>>()),
        };
        if let Some(errors) = &errors {
            println!("Error while validating event:\n{:#?}", event);
            for error in errors {
                println!("Error: {:#?}", error);
            }
        }
        assert!(errors.is_none());
    }
}
