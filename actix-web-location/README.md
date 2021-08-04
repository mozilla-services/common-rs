# actix-web-location

[![License: MPL 2.0]][mpl 2.0] [![Build Status]][circleci]
[![version-badge::actix-web-location]][crates.io::actix-web-location]
[![rustdoc-badge::actix-web-location]][docs::actix-web-location]

[license: mpl 2.0]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl 2.0]: https://opensource.org/licenses/MPL-2.0
[build status]:
  https://img.shields.io/circleci/build/github/mozilla-services/common-rs
[circleci]: https://app.circleci.com/pipelines/github/mozilla-services/common-rs
[version-badge::actix-web-location]:
  https://img.shields.io/crates/v/actix-web-location.svg
[crates.io::actix-web-location]: https://crates.io/cratesactix-web-location/
[docs::actix-web-location]: https://docs.rsactix-web-location/
[rustdoc-badge::actix-web-location]:
  https://img.shields.io/docsrsactix-web-location/

A extensible crate to provide location determination for [actix-web], using
GeoIP or other techniques.

This crate is optionally compatible with actix-web version 4. By default version
3 is supported, but by setting the feature `actix-web-4`, the crate will switch
to version 4. Only one is supported at a time.

[tracing]: https://tracing.rs/tracing/
[actix-web]: https://actix.rs/
[mozlog]: https://wiki.mozilla.org/Firefox/Services/Logging
