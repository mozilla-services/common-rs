//! Providers are `actix-web-location`'s abstraction to allow multiple ways of determining location.

use crate::{domain::LocationBuilder, Error, Location};
use async_trait::async_trait;
#[cfg(feature = "maxmind")]
pub use maxmind::MaxMindProvider;

#[cfg(not(feature = "actix-web-v4"))]
use actix_web_3::HttpRequest;
#[cfg(feature = "actix-web-v4")]
use actix_web_4::HttpRequest;

/// An object that can be queried to convert [`HttpRequest`] into locations.
///
/// Use [`macro@async_trait`] when implementing.
#[async_trait(?Send)]
pub trait Provider: Send + Sync {
    /// Provide a name of the provider for use in diagnostics.
    fn name(&self) -> &str;

    /// Derive a location from a request's metadata.
    async fn get_location(&self, request: &HttpRequest) -> Result<Option<Location>, Error>;

    /// Can this provider produce locations with country information?
    fn expect_country(&self) -> bool {
        true
    }

    /// Can this provider produce locations with region information?
    fn expect_region(&self) -> bool {
        true
    }

    /// Can this provider produce locations with city information?
    fn expect_city(&self) -> bool {
        true
    }
}

/// A "dummy" provider that returns None for all fields.
pub struct FallbackProvider {
    fallback: Location,
}

impl FallbackProvider {
    /// Create a fallback provider.
    ///
    /// The passed location builder will be modified to include the provider name.
    pub fn new(fallback_builder: LocationBuilder) -> Self {
        Self {
            fallback: fallback_builder
                .provider("fallback".to_string())
                .finish()
                .expect("Location construction bug"),
        }
    }
}

#[async_trait(?Send)]
impl Provider for FallbackProvider {
    fn name(&self) -> &str {
        "fallback"
    }

    async fn get_location(&self, _request: &HttpRequest) -> Result<Option<Location>, Error> {
        Ok(Some(self.fallback.clone()))
    }
}

#[cfg(feature = "maxmind")]
mod maxmind {
    use std::{
        net::{IpAddr, SocketAddr},
        path::Path,
        sync::Arc,
    };

    use crate::domain::LocationBuilder;

    use super::{Error, Location, Provider};
    use anyhow::anyhow;
    use async_trait::async_trait;
    use lazy_static::lazy_static;
    use maxminddb::geoip2::City;

    #[cfg(not(feature = "actix-web-v4"))]
    use actix_web_3::{http::HeaderName, HttpRequest};
    #[cfg(feature = "actix-web-v4")]
    use actix_web_4::{http::HeaderName, HttpRequest};

    lazy_static! {
        static ref X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
    }

    /// A provider that uses a MaxMind GeoIP database to derive location from a the IP a request was sent from.
    #[derive(Clone)]
    pub struct MaxMindProvider {
        mmdb: Arc<maxminddb::Reader<Vec<u8>>>,
    }

    impl MaxMindProvider {
        /// Read a file from the given path into memory, and use it to construct a location provider.
        pub fn from_path(path: &Path) -> Result<Self, Error> {
            Ok(Self {
                mmdb: maxminddb::Reader::open_readfile(path)
                    .map_err(|e| Error::Setup(anyhow!("{}", e)))
                    .map(Arc::new)?,
            })
        }
    }

    #[async_trait(?Send)]
    impl Provider for MaxMindProvider {
        fn name(&self) -> &str {
            "maxmind"
        }

        async fn get_location(&self, request: &HttpRequest) -> Result<Option<Location>, Error> {
            let header = request.headers().get(&*X_FORWARDED_FOR);

            let addr = if let Some(header) = header {
                // Expect a typical X-Forwarded-For where the first address is
                // the client's, the front ends should ensure this
                let value = header
                    .to_str()
                    .map_err(|e| Error::Http(e.into()))?
                    .split(',')
                    .next()
                    .unwrap_or_default()
                    .trim();
                let parsed = value
                    .parse::<IpAddr>()
                    // Fallback to parsing as SocketAddr for when a port
                    // number's included
                    .or_else(|_| value.parse::<SocketAddr>().map(|socket| socket.ip()))
                    .map_err(|e| Error::Http(e.into()))?;
                Some(parsed)
            } else {
                None
            };

            addr.map(|addr| {
                let city = self
                    .mmdb
                    .lookup::<City>(addr)
                    .map_err(|err| Error::Provider(err.into()))?;
                let builder: LocationBuilder = (city, "en").into();
                builder
                    .provider("maxmind".to_string())
                    .finish()
                    .map_err(|_| Error::Provider(anyhow::anyhow!("Bug while building location")))
            })
            .transpose()
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #[cfg(not(feature = "actix-web-v4"))]
    use actix_web_3::test::TestRequest;
    #[cfg(feature = "actix-web-v4")]
    use actix_web_4::test::TestRequest;

    use super::FallbackProvider;
    use crate::{Location, Provider};

    #[actix_rt::test]
    async fn fallback_works_empty() {
        let provider = FallbackProvider::new(Location::build());
        let request = TestRequest::default().to_http_request();
        let location = provider
            .get_location(&request)
            .await
            .expect("Could not get location")
            .expect("Location was none");
        assert_eq!(
            location,
            Location {
                country: None,
                region: None,
                city: None,
                dma: None,
                provider: "fallback".to_string()
            }
        )
    }

    #[actix_rt::test]
    async fn fallback_works_full() {
        let provider = FallbackProvider::new(
            Location::build()
                .country("CA".to_string())
                .region("BC".to_string())
                .city("Burnaby".to_string()),
        );
        let request = TestRequest::default().to_http_request();
        let location = provider
            .get_location(&request)
            .await
            .expect("Could not get location")
            .expect("Location was none");
        assert_eq!(
            location,
            Location {
                country: Some("CA".to_string()),
                region: Some("BC".to_string()),
                city: Some("Burnaby".to_string()),
                dma: None,
                provider: "fallback".to_string()
            }
        )
    }

    #[cfg(feature = "maxmind")]
    pub(crate) mod maxmind {
        use std::path::PathBuf;

        use crate::{providers::MaxMindProvider, Error, Location, Provider};

        #[cfg(not(feature = "actix-web-v4"))]
        use actix_web_3::test::TestRequest;
        #[cfg(feature = "actix-web-v4")]
        use actix_web_4::test::TestRequest;

        pub(crate) const MMDB_LOC: &str = "./GeoLite2-City-Test.mmdb";
        pub(crate) const TEST_ADDR_1: &str = "216.160.83.56";
        pub(crate) const TEST_ADDR_2: &str = "127.0.0.1";
        pub(crate) const TEST_ADDR_3: &str = "216.160.83.56, 127.0.0.1, 10.0.0.1";
        pub(crate) const TEST_ADDR_4: &str = "216.160.83.56:31337, 127.0.0.1";

        /// Return the expected location for [TEST_ADDR_1]
        fn test_location() -> Location {
            Location::build()
                .country("US".to_string())
                .region("WA".to_string())
                .city("Milton".to_string())
                .dma(819)
                .provider("maxmind".to_string())
                .finish()
                .expect("bug when creating location")
        }

        #[actix_rt::test]
        async fn known_ip() {
            let provider = MaxMindProvider::from_path(&PathBuf::from(MMDB_LOC))
                .expect("could not make maxmind client");

            #[cfg(not(feature = "actix-web-v4"))]
            let request = TestRequest::default()
                .header("X-Forwarded-For", TEST_ADDR_1)
                .to_http_request();
            #[cfg(feature = "actix-web-v4")]
            let request = TestRequest::default()
                .insert_header(("X-Forwarded-For", TEST_ADDR_1))
                .to_http_request();

            let location = provider
                .get_location(&request)
                .await
                .expect("could not get location")
                .expect("location was none");
            assert_eq!(location, test_location());
        }

        #[actix_rt::test]
        async fn unknown_ip() {
            let provider = MaxMindProvider::from_path(&PathBuf::from(MMDB_LOC))
                .expect("could not make maxmind client");

            #[cfg(not(feature = "actix-web-v4"))]
            let request = TestRequest::default()
                .header("X-Forwarded-For", TEST_ADDR_2)
                .to_http_request();
            #[cfg(feature = "actix-web-v4")]
            let request = TestRequest::default()
                .insert_header(("X-Forwarded-For", TEST_ADDR_2))
                .to_http_request();

            let location = provider.get_location(&request).await;
            assert!(matches!(location, Err(Error::Provider(_))));
        }

        #[actix_rt::test]
        async fn with_proxy_ips() {
            let provider = MaxMindProvider::from_path(&PathBuf::from(MMDB_LOC))
                .expect("could not make maxmind client");

            #[cfg(not(feature = "actix-web-v4"))]
            let request = TestRequest::default()
                .header("X-Forwarded-For", TEST_ADDR_3)
                .to_http_request();
            #[cfg(feature = "actix-web-v4")]
            let request = TestRequest::default()
                .insert_header(("X-Forwarded-For", TEST_ADDR_3))
                .to_http_request();

            let location = provider
                .get_location(&request)
                .await
                .expect("could not get location")
                .expect("location was none");
            assert_eq!(location, test_location());
        }

        #[actix_rt::test]
        async fn with_port() {
            let provider = MaxMindProvider::from_path(&PathBuf::from(MMDB_LOC))
                .expect("could not make maxmind client");

            #[cfg(not(feature = "actix-web-v4"))]
            let request = TestRequest::default()
                .header("X-Forwarded-For", TEST_ADDR_4)
                .to_http_request();
            #[cfg(feature = "actix-web-v4")]
            let request = TestRequest::default()
                .insert_header(("X-Forwarded-For", TEST_ADDR_4))
                .to_http_request();

            let location = provider
                .get_location(&request)
                .await
                .expect("could not get location")
                .expect("location was none");
            assert_eq!(location, test_location());
        }

        #[test]
        fn expected_info() {
            let provider = MaxMindProvider::from_path(&PathBuf::from(MMDB_LOC))
                .expect("could not make maxmind client");
            assert!(provider.expect_country());
            assert!(provider.expect_region());
            assert!(provider.expect_city());
        }
    }
}
