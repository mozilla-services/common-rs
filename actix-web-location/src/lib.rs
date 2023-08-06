//! Location determination for Actix Web applications.

#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

/* This is a mess, but it makes the rest of the crate tidier. Only include this
 * modules and uses if exactly one of v3 or v4 is specified. */

#[cfg(any(
    all(feature = "actix-web-v3", not(feature = "actix-web-v4")),
    all(not(feature = "actix-web-v3"), feature = "actix-web-v4")
))]
mod domain;
#[cfg(any(
    all(feature = "actix-web-v3", not(feature = "actix-web-v4")),
    all(not(feature = "actix-web-v3"), feature = "actix-web-v4")
))]
mod error;
#[cfg(any(
    all(feature = "actix-web-v3", not(feature = "actix-web-v4")),
    all(not(feature = "actix-web-v3"), feature = "actix-web-v4")
))]
mod extractors;
#[cfg(any(
    all(feature = "actix-web-v3", not(feature = "actix-web-v4")),
    all(not(feature = "actix-web-v3"), feature = "actix-web-v4")
))]
pub mod providers;

#[cfg(any(
    all(feature = "actix-web-v3", not(feature = "actix-web-v4")),
    all(not(feature = "actix-web-v3"), feature = "actix-web-v4")
))]
pub use crate::{domain::Location, error::Error, extractors::LocationConfig, providers::Provider};

/* The two stanzas below provide nicer error messages if not exactly one of v3
 * and v4 are enabled. They aren't hard errors so that this crate's CI still
 * works, but because nothing will be defined above it will be a hard error for
 * any downstream crates. This provides a nicer message to debug what's happening.
 * It uses deprecation notices because this is the only way to generate compiler
 * warnings.
 * */

/* If both v3 and v4 are enabled at the same time, generate a compiler warning. */
#[cfg(all(feature = "actix-web-v3", feature = "actix-web-v4"))]
mod warning {
    #![allow(dead_code)]

    #[deprecated(
        note = "Only one of actix-web-v3 and actix-web-v4 can be used at once. This entire crate will be disabled."
    )]
    fn actix_web_location_must_specify_one_version_of_actix_web() {}
    fn trigger_warning() {
        actix_web_location_must_specify_one_version_of_actix_web()
    }
}

/* If neither v3 or v4 are enabled at the same time, generate a compiler warning. */
#[cfg(not(any(feature = "actix-web-v3", feature = "actix-web-v4")))]
mod warning {
    #![allow(dead_code, deprecated)]

    #[deprecated(note = "Exactly of actix-web-v3 or actix-web-v4 must be specified")]
    fn actix_web_location_must_specify_one_version_of_actix_web() {}
    fn trigger_warning() {
        actix_web_location_must_specify_one_version_of_actix_web()
    }
}
