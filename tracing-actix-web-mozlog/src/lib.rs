//! # tracing-actix-web-mozlog
//!
//! Support for [tracing] in [actix-web](https://actix.rs/) apps that target [MozLog][].
//!
//! [MozLog]: https://wiki.mozilla.org/Firefox/Services/Logging
//!
//! This crate provides a Tracing subscriber as well as an Actix web middleware.
//! Both can be used independently, but using them together implements the full
//! recommended format for MozLog.
//!
//! ## Subscriber
//!
//! To use the subscriber, register it and a [`JsonStorageLayer`] with a
//! [`tracing_subscriber::Registry`]:
//!
//! ```rust
//! use tracing_actix_web_mozlog::{JsonStorageLayer, MozLogFormatLayer};
//! use tracing_subscriber::layer::SubscriberExt;
//!
//! let subscriber = tracing_subscriber::registry()
//!     .with(JsonStorageLayer)
//!     .with(MozLogFormatLayer::new("service-name", std::io::stdout));
//! ```
//!
//! This subscriber can then be registered with tracing using
//! [`tracing::subscriber::set_global_default`], or any other registration
//! method. It will manage formatting any events logged in MozLog JSON format.
//!
//! Fields defined on the enclosing spans of an event will be included when
//! logging an event. The event overrides the spans, and inner spans override
//! outer spans.
//!
//! ## Middleware
//!
//! To make sure that the correct Tracing context is captured, it is important to
//! create the MozLog middleware outside of the `HttpServer::new` closure:
//!
//! ```rust
//! use tracing_actix_web_mozlog::MozLog;
//! use actix_web::{HttpServer, App};
//!
//! let moz_log = MozLog::default();
//!
//! let server = HttpServer::new(move || {
//!     App::new()
//!         .wrap(moz_log.clone())
//! });
//! ```
//!
//! This middleware will emit `request.summary` events for each request as it is
//! completed, including timing information.
//!
//! ## Message Types
//!
//! MozLog expects all messages to have a type that defines the schema of their
//! fields. This can be specified with the `type` field while logging events.
//! Since `type` is a Rust reserved keyword, this can also be specified using a
//! raw-string-inspired format: `r#type = value`.
//!
//! ```rust
//! let format_error = "...";
//! tracing::warn!(
//!     r#type = "auth.login.invalid-email",
//!     %format_error,
//!     "A user attempted to register using an email in an invalid format"
//! );
//! ```
//!
//! ### Fallback message types
//!
//! Messages that don't include a `type` field will be assigned a type of
//! `<unknown>`. If a message contains both a `type` field and a `r#type` field,
//! the `type` field will take precedence.
//!
//! Notably, use of the standard `log` facade's macros will have an unknown type,
//! as well as most other logging that originates from libraries.
//!
//! ### Enforcing messages
//! You can optionally make log messages that lack a `type` field also emit
//! errors, depending on their level. For example, to make all messages of
//! ERROR, WARN, and INFO require a `type` field:
//!
//! ```rust
//! use tracing_actix_web_mozlog::MozLogFormatLayer;
//!
//! MozLogFormatLayer::new("service-name", std::io::stdout)
//!     .with_type_required_for_level(Some(tracing::Level::INFO));
//! ```
//!
//! ## MozLog extensions
//!
//! In addition to all standard MozLog fields, this crate always adds a `spans`
//! field to messages. This contains a comma-separated list of the names of the
//! spans enclosing the event, with the outermost span coming first. Top-level
//! events will have an empty string for this value.

#![warn(missing_crate_level_docs)]
#![warn(missing_docs)]

mod middleware;
mod subscriber;

pub use crate::middleware::MozLog;
pub use crate::subscriber::{MozLogFormatLayer, MozLogMessage};

/// A layer to collect information about Tracing spans and provide it to other layers.
///
/// This is a re-exported entry from [`tracing_bunyan_formatter`].
pub use tracing_bunyan_formatter::JsonStorageLayer;
