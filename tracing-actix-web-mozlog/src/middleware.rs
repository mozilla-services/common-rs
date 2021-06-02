//! Loggers for the request/response cycle.

use std::time::Instant;

use actix_web::{dev::ServiceResponse, HttpMessage};
use tracing::Span;
use tracing_actix_web::{RequestId, RootSpanBuilder, TracingLogger};

/// Middleware that implements the request/response cycle logging required by MozLog.
pub type MozLogMiddleware = TracingLogger<MozLogRootSpanBuilder>;

/// A root span builder for tracing_actix_web to customize the extra fields we
/// log with requests, and to log an event when requests end.
///
/// # Examples
///
/// ```
/// use tracing_actix_web_mozlog::MozLogMiddleware;
/// use actix_web::App;
/// App::new()
///     .wrap(MozLogMiddleware::new());
/// ```
pub struct MozLogRootSpanBuilder;

struct RequestStart(Instant);

impl RootSpanBuilder for MozLogRootSpanBuilder {
    fn on_request_start(request: &actix_web::dev::ServiceRequest) -> tracing::Span {
        let http_method = request.method().as_str();

        let mut request_extensions = request.extensions_mut();
        let request_id = request_extensions.get::<RequestId>().cloned().unwrap();
        request_extensions.insert(RequestStart(Instant::now()));

        let span = tracing::info_span!(
            "request",
            method = %http_method,
            path = %request.uri().path_and_query().map(|p| p.as_str()).unwrap_or(""),
            code = tracing::field::Empty,
            rid = %request_id,
            errno = tracing::field::Empty,
            agent = tracing::field::Empty,
            msg = tracing::field::Empty,
            lang = tracing::field::Empty,
            uid = tracing::field::Empty,
            t = tracing::field::Empty,
            t_ns = tracing::field::Empty,
        );

        if let Some(user_agent) = request.headers().get("User-Agent") {
            span.record("agent", &user_agent.to_str().unwrap_or("<bad_utf8>"));
        }

        span
    }

    fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, actix_web::Error>) {
        match &outcome {
            Ok(response) => {
                if let Some(req_start) = response.request().extensions().get::<RequestStart>() {
                    let elapsed = req_start.0.elapsed();
                    span.record("t", &(elapsed.as_millis() as u32));
                    span.record("t_ns", &(elapsed.as_nanos() as u64));
                }

                if let Some(error) = response.response().error() {
                    handle_error(span, error);
                } else {
                    span.record("code", &response.response().status().as_u16());
                    response.status();
                }
            }
            Err(error) => handle_error(span, error),
        };

        tracing::info!(r#type = "request.summary")
    }
}

/// Annotate the root request span with information about a request error.
fn handle_error(span: Span, error: &actix_web::Error) {
    let response_error = error.as_response_error();
    let status = response_error.status_code();
    span.record("errno", &1);
    span.record("msg", &tracing::field::display(response_error));
    span.record("code", &status.as_u16());
}
