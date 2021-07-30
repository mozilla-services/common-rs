//! Location determination for Actix Web applications.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod domain;
mod error;
mod extractors;
pub mod providers;

pub use crate::{domain::Location, error::Error, extractors::LocationConfig, providers::Provider};
