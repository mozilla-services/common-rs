use std::sync::Arc;

use crate::{domain::Location, error::Error, providers::Provider};
use anyhow::anyhow;
use futures::{future::LocalBoxFuture, FutureExt};
use lazy_static::lazy_static;

#[cfg(feature = "actix-web-v3")]
use actix_web_3::{dev, web, FromRequest, HttpRequest};

#[cfg(feature = "actix-web-v4")]
use actix_web_4::{dev, web, FromRequest, HttpRequest};

impl FromRequest for Location {
    #[cfg(feature = "actix-web-v3")]
    type Config = LocationConfig;

    type Error = Error;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        async move {
            let config = LocationConfig::from_req(&req).clone();
            let mut result: Option<Result<Self, Self::Error>> = None;
            for provider in config.providers {
                if let Ok(Some(location)) = provider.get_location(&req).await {
                    #[cfg(feature = "cadence")]
                    {
                        if let Some(metrics) = config.metrics.as_ref() {
                            if provider.expect_city() && location.city.is_none() {
                                metrics
                                    .incr_with_tags("location.unknown.city")
                                    .with_tag("provider", provider.name())
                                    .try_send()
                                    .ok();
                            }
                            if provider.expect_region() && location.region.is_none() {
                                metrics
                                    .incr_with_tags("location.unknown.region")
                                    .with_tag("provider", provider.name())
                                    .try_send()
                                    .ok();
                            }
                            if provider.expect_country() && location.country.is_none() {
                                metrics
                                    .incr_with_tags("location.unknown.country")
                                    .with_tag("provider", provider.name())
                                    .try_send()
                                    .ok();
                            }
                        }
                    }

                    result = Some(Ok(location));

                    break;
                }
            }

            #[cfg(feature = "cadence")]
            let metrics = config.metrics.as_ref();

            result.unwrap_or_else(|| {
                #[cfg(feature = "cadence")]
                {
                    if let Some(metrics) = metrics {
                        metrics
                            .incr_with_tags("location.unknown.city")
                            .with_tag("provider", "none")
                            .try_send()
                            .ok();
                        metrics
                            .incr_with_tags("location.unknown.region")
                            .with_tag("provider", "none")
                            .try_send()
                            .ok();
                        metrics
                            .incr_with_tags("location.unknown.country")
                            .with_tag("provider", "none")
                            .try_send()
                            .ok();
                    }
                }

                Location::build()
                    .provider("none".to_string())
                    .finish()
                    .map_err(|_| Error::Http(anyhow!("Bug when processing default result")))
            })
        }
        .boxed_local()
    }
}

/// Configuration for how to determine location from a request.
#[derive(Clone, Default)]
pub struct LocationConfig {
    /// The provider to request location information from.
    providers: Vec<Arc<Box<dyn Provider>>>,

    /// An optional sink to send metrics to.
    #[cfg(feature = "cadence")]
    metrics: Option<Arc<dyn cadence::CountedExt + Send + Sync>>,
}

lazy_static! {
    static ref DEFAULT_LOCATION_CONFIG: LocationConfig = LocationConfig::default();
}

impl LocationConfig {
    /// Add a provider to this configuration. It will be wrapped into an `Arc<Box<T>>`.
    pub fn with_provider<P: Provider + 'static>(mut self, provider: P) -> Self {
        self.providers.push(Arc::new(Box::new(provider)));
        self
    }

    /// Add a metrics sink to this configuration. It will be wrapped into an `Arc<Option<Box<T>>>`.
    #[cfg(feature = "cadence")]
    pub fn with_metrics<M: cadence::CountedExt + Send + Sync + 'static>(
        mut self,
        metrics: Arc<M>,
    ) -> Self {
        self.metrics = Some(metrics);
        self
    }

    fn from_req(req: &HttpRequest) -> &Self {
        req.app_data::<Self>()
            .or_else(|| req.app_data::<web::Data<Self>>().map(|d| d.as_ref()))
            .unwrap_or(&DEFAULT_LOCATION_CONFIG)
    }
}

#[cfg(test)]
mod tests {
    use crate::{providers::FallbackProvider, Location, LocationConfig};

    #[cfg(not(feature = "actix-web-v4"))]
    use actix_web_3::{dev::Payload, test::TestRequest, FromRequest};
    #[cfg(feature = "actix-web-v4")]
    use actix_web_4::{dev::Payload, test::TestRequest, FromRequest};

    #[actix_rt::test]
    async fn default_config() {
        let req = TestRequest::default()
            .app_data(LocationConfig::default())
            .to_http_request();
        let location = Location::from_request(&req, &mut Payload::None)
            .await
            .expect("error getting request");
        assert_eq!(
            location,
            Location {
                country: None,
                region: None,
                city: None,
                dma: None,
                provider: "none".to_string()
            }
        );
    }

    #[actix_rt::test]
    async fn with_provider() {
        let provider = FallbackProvider::new(
            Location::build()
                .country("CA".to_string())
                .region("ON".to_string())
                .city("Toronto".to_string()),
        );
        let config = LocationConfig::default().with_provider(provider);
        let req = TestRequest::default().app_data(config).to_http_request();
        let location = Location::from_request(&req, &mut Payload::None)
            .await
            .expect("error getting request");
        assert_eq!(
            location,
            Location {
                country: Some("CA".to_string()),
                region: Some("ON".to_string()),
                city: Some("Toronto".to_string()),
                dma: None,
                provider: "fallback".to_string()
            }
        );
    }

    // TODO test metrics
}
