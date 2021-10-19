use thiserror::Error;

#[cfg(feature = "actix-web-v3")]
use actix_web_3::ResponseError;
#[cfg(feature = "actix-web-v4")]
use actix_web_4::ResponseError;

/// An error that occurred while providing a location.
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("there was an error while setting up a provider")]
    Setup(#[source] anyhow::Error),

    #[error("there was an error accessing an underlying provider")]
    Provider(#[source] anyhow::Error),

    #[error("problem with the HTTP request")]
    Http(#[source] anyhow::Error),

    #[error("problem converting provider response to a location")]
    Conversion(#[source] anyhow::Error),
}

impl ResponseError for Error {}
