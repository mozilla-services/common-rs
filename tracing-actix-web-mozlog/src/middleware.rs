//! Loggers for the request/response cycle.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    HttpMessage,
};
use tracing::{Dispatch, Span};
use tracing_actix_web::{RequestId, RootSpanBuilder, TracingLogger};
use tracing_futures::WithSubscriber;

/// Middleware factory that implements the request/response cycle logging
/// required by MozLog.
///
/// To make sure that the correct Tracing context is captured, it is important to
/// create the MozLog middleware outside of the `HttpServer::new` closure. The
/// middleware will capture the current Tracing subscriber, and make sure to
/// apply it to each worker as needed.
///
/// ```
/// use tracing_actix_web_mozlog::MozLog;
/// use actix_web::{HttpServer, App};
///
/// let moz_log = MozLog::default();
///
/// let server = HttpServer::new(move || {
///     App::new()
///         .wrap(moz_log.clone())
/// });
/// ```
///
/// This middleware will emit `request.summary` events for each request as it is
/// completed, including timing information.
#[derive(Clone)]
pub struct MozLog {
    dispatch: Dispatch,
    tracing_logger: TracingLogger<MozLogRootSpanBuilder>,
}

impl Default for MozLog {
    fn default() -> Self {
        let mut dispatch = None;
        tracing::dispatcher::get_default(|d| dispatch = Some(d.clone()));
        Self {
            dispatch: dispatch.unwrap(),
            tracing_logger: TracingLogger::new(),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for MozLog
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static + MessageBody,
    S: 'static,
{
    type Response =
        <TracingLogger<MozLogRootSpanBuilder> as Transform<S, ServiceRequest>>::Response;
    type Error = actix_web::Error;
    type Transform = MozLogMiddleware<
        <TracingLogger<MozLogRootSpanBuilder> as Transform<S, ServiceRequest>>::Transform,
    >;
    type InitError = ();
    type Future = MozLogTransform<S, B>;

    fn new_transform(&self, service: S) -> Self::Future {
        MozLogTransform {
            inner: Box::pin(self.tracing_logger.new_transform(service)),
            dispatch: self.dispatch.clone(),
        }
    }
}

type TracingLoggerMiddleware<S> =
    <TracingLogger<MozLogRootSpanBuilder> as Transform<S, ServiceRequest>>::Transform;

type ServiceFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>>>>;

pub struct MozLogTransform<S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static + MessageBody,
{
    dispatch: Dispatch,
    inner: Pin<Box<dyn Future<Output = Result<TracingLoggerMiddleware<S>, ()>>>>,
}

impl<S, B> Future for MozLogTransform<S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static + MessageBody,
{
    type Output = Result<MozLogMiddleware<TracingLoggerMiddleware<S>>, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(Ok(inner)) => Poll::Ready(Ok(MozLogMiddleware {
                service: inner,
                dispatch: self.dispatch.clone(),
            })),
            Poll::Ready(Err(_)) => Poll::Ready(Err(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct MozLogMiddleware<S> {
    service: S,
    dispatch: Dispatch,
}

impl<S, B> Service<ServiceRequest> for MozLogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = ServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        Box::pin(
            self.service
                .call(req)
                .with_subscriber(self.dispatch.clone()),
        )
    }
}

/// A root span builder for tracing_actix_web to customize the extra fields we
/// log with requests, and to log an event when requests end.
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
            path = %request.uri().path(),
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
            span.record("agent", user_agent.to_str().unwrap_or("<bad_utf8>"));
        }

        span
    }

    fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, actix_web::Error>) {
        match &outcome {
            Ok(response) => {
                if let Some(req_start) = response.request().extensions().get::<RequestStart>() {
                    let elapsed = req_start.0.elapsed();
                    span.record("t", elapsed.as_millis() as u32);
                    span.record("t_ns", elapsed.as_nanos() as u64);
                }

                if let Some(error) = response.response().error() {
                    handle_error(span, error);
                } else {
                    span.record("code", response.response().status().as_u16());
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
    span.record("errno", 1);
    span.record("msg", &tracing::field::display(response_error));
    span.record("code", status.as_u16());
}
