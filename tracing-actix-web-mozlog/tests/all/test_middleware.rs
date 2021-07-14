use actix_service::Service;
use actix_web::{get, http::StatusCode, test, web, App, HttpResponse, ResponseError};
use maplit::hashmap;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::fmt::Display;

use crate::utils::{log_test_async, LogWatcher};
use tracing_actix_web_mozlog::{MozLog, MozLogMessage};

#[get("/{status}")]
async fn handler_status_echo(status: web::Path<u16>) -> HttpResponse {
    HttpResponse::new(StatusCode::from_u16(*status).expect("invalid status code"))
}

#[derive(Debug)]
struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}

impl ResponseError for TestError {}

#[get("/")]
async fn handler_error() -> Result<HttpResponse, TestError> {
    Err(TestError)
}

#[actix_rt::test]
async fn test_it_logs_requests() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let middleware = MozLog::default();
        let app =
            test::init_service(App::new().wrap(middleware).service(handler_status_echo)).await;

        let req = test::TestRequest::with_uri("/200").to_request();
        let res = app.call(req).await.expect("request handler error");
        assert_eq!(res.status(), StatusCode::OK);

        let req = test::TestRequest::with_uri("/400").to_request();
        let res = app.call(req).await.expect("request handler error");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let req = test::TestRequest::with_uri("/500").to_request();
        let res = app.call(req).await.expect("request handler error");
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;

    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.message_type == "request.summary"
                && event.fields.get("code") == Some(&json!(200))
        }),
        "should log successful responses"
    );
    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.fields.get("code") == Some(&json!(400))
                && event.message_type == "request.summary"
        }),
        "should log client errors"
    );
    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.fields.get("code") == Some(&json!(500))
                && event.message_type == "request.summary"
        }),
        "should log server errors"
    );
}

#[actix_rt::test]
async fn test_request_summary_has_recommended_fields() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let middleware = MozLog::default();
        let app =
            test::init_service(App::new().wrap(middleware).service(handler_status_echo)).await;

        let req = test::TestRequest::with_uri("/200")
            .append_header(("User-Agent", "A Test Client"))
            .to_request();
        let res = app.call(req).await.expect("request handler error");
        assert_eq!(res.status(), StatusCode::OK);
    })
    .await;

    let event = dbg!(log_watcher.events())
        .iter()
        .find(|event| event.message_type == "request.summary")
        .expect("Could not find request.summary event");
    assert_eq!(
        *event,
        MozLogMessage {
            message_type: "request.summary".into(),
            logger: "test-logger".into(),
            env_version: "2.0".into(),
            severity: 5,
            fields: hashmap! {
                "agent".to_string() => json!("A Test Client"),
                "path".to_string() => json!("/200"),
                "method".to_string() => json!("GET"),
                "code".to_string() => json!(200),
                "spans".to_string() => json!("request"),
                "rid".to_string() => event.fields.get("rid")
                    .expect("Should have a request id").clone(),
                "t".to_string() => event.fields.get("t")
                    .expect("should have request time in milliseconds").clone(),
                "t_ns".to_string() => event.fields.get("t_ns")
                    .expect("should have request time in nanoseconds").clone(),
            },
            ..event.clone()
        },
        "Should have the expected fields"
    );
}

#[actix_rt::test]
async fn test_it_logs_controlled_errors() {
    let mut log_watcher: LogWatcher = log_test_async(|| async {
        let middleware = MozLog::default();
        let app = test::init_service(App::new().wrap(middleware).service(handler_error)).await;
        let req = test::TestRequest::with_uri("/").to_request();
        let res = app.call(req).await.expect("request handler error");
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    })
    .await;

    assert!(
        log_watcher.has(|event| {
            event.severity == 5
                && event.message_type == "request.summary"
                && event.fields.get("code") == Some(&json!(500))
        }),
        "errors are still logged with INFO level request.summary"
    );
}
