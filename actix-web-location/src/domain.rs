#[cfg(feature = "maxmind")]
use maxminddb::geoip2::City;
#[cfg(feature = "serde")]
use serde::Serialize;

/// The location information that providers must produce.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Location {
    /// Country in ISO 3166-1 alpha-2 format, such as "MX" for Mexico or "IT" for Italy.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub country: Option<String>,

    /// Region/region (e.g. a US state) in ISO 3166-2 format, such as "QC"
    /// for Quebec (with country = "CA") or "TX" for Texas (with country = "US").
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub region: Option<String>,

    /// City, listed by name such as "Portland" or "Berlin".
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub city: Option<String>,

    /// The Designated Market Area code, as defined by [Nielsen]. Only defined in the US.
    ///
    /// [Nielsen]: https://www.nielsen.com/us/en/contact-us/intl-campaigns/dma-maps/
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub dma: Option<u16>,

    /// The name of the provider that produced this recommendation.
    pub provider: String,
}

macro_rules! location_field {
    ($field: ident, $type: ty) => {
        location_field!(
            $field,
            $type,
            concat!(
                "Get an owned copy of the ",
                stringify!($field),
                ", or the default if the field is None"
            )
        );
    };

    ($field: ident, $type: ty, $doc: expr) => {
        #[doc = $doc]
        pub fn $field(&self) -> $type {
            self.$field.clone().unwrap_or_default()
        }
    };
}

impl Location {
    /// Create a builder for a [`Location`] that can be assembled incrementally.
    pub fn build() -> LocationBuilder {
        LocationBuilder::default()
    }

    location_field!(country, String);
    location_field!(region, String);
    location_field!(city, String);
    location_field!(dma, u16);
}

#[derive(Default)]
pub struct LocationBuilder {
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
    dma: Option<u16>,
    provider: Option<String>,
}

macro_rules! builder_field {
    ($field: ident, $type: ty) => {
        pub fn $field<O: Into<Option<$type>>>(mut self, $field: O) -> Self {
            self.$field = $field.into();
            self
        }
    };
}

impl LocationBuilder {
    builder_field!(country, String);
    builder_field!(region, String);
    builder_field!(city, String);
    builder_field!(dma, u16);
    builder_field!(provider, String);

    pub fn finish(self) -> Result<Location, ()> {
        Ok(Location {
            country: self.country,
            region: self.region,
            city: self.city,
            dma: self.dma,
            provider: self.provider.ok_or(())?,
        })
    }
}

#[cfg(feature = "maxmind")]
impl<'a> From<City<'a>> for LocationBuilder {
    fn from(val: City<'a>) -> Self {
        Location::build()
            .country(
                val.country
                    .and_then(|country| country.iso_code)
                    .map(String::from),
            )
            .region(
                val.subdivisions
                    // Subdivisions are listed in least-specific order. In the US, this might mean that subdivisions is state and then county. We want only the first.
                    .and_then(|subdivisions| {
                        subdivisions
                            .get(0)
                            .and_then(|subdivision| subdivision.iso_code)
                    })
                    .map(ToString::to_string),
            )
            .city(
                val.city
                    .and_then(|city| city.names)
                    .and_then(|names| names.get("en").map(|name| name.to_string()))
                    .map(|name| (*name).to_string()),
            )
            .dma(val.location.and_then(|location| location.metro_code))
    }
}

#[cfg(test)]
mod tests {
    use super::Location;

    #[test]
    fn builder_works() {
        let location = Location::build()
            .country("US".to_string())
            .region("OR".to_string())
            .city("Portland".to_string())
            .dma(810)
            .provider("test".to_string())
            .finish()
            .unwrap();

        assert_eq!(
            location,
            Location {
                country: Some("US".to_string()),
                region: Some("OR".to_string()),
                city: Some("Portland".to_string()),
                dma: Some(810),
                provider: "test".to_string()
            }
        );
    }

    #[test]
    fn methods_get_values() {
        let location = Location::build()
            .country("US".to_string())
            .region("CA".to_string())
            .city("Sunnyvale".to_string())
            .dma(807)
            .provider("test".to_string())
            .finish()
            .unwrap();

        assert_eq!(location.country(), "US");
        assert_eq!(location.region(), "CA");
        assert_eq!(location.city(), "Sunnyvale");
        assert_eq!(location.dma(), 807);
    }

    #[test]
    fn methods_get_defaults() {
        let location = Location::build()
            .provider("test".to_string())
            .finish()
            .unwrap();

        assert_eq!(location.country(), "");
        assert_eq!(location.region(), "");
        assert_eq!(location.city(), "");
        assert_eq!(location.dma(), 0);
    }
}
