[![License: MPL 2.0]][mpl 2.0] [![Build Status]][travis]
[![Latest Version]][crates.io] [![Api Rustdoc]][rustdoc]

[license: mpl 2.0]: https://img.shields.io/badge/License-MPL%202.0-blue.svg
[mpl 2.0]: https://opensource.org/licenses/MPL-2.0
[build status]:
  https://travis-ci.org/mozilla-services/common-rs.svg?branch=master
[travis]: https://travis-ci.org/mozilla-services/common-rs
[latest version]: https://img.shields.io/crates/v/mozsvc-common.svg
[crates.io]: https://crates.io/crates/mozsvc-common
[api rustdoc]: https://img.shields.io/badge/api-rustdoc-blue.svg
[rustdoc]: https://docs.rs/mozsvc-common

A common set of utilities for Mozilla server side applications.

This repo is structured as a workspace, and each crate is expected to be
published.

# Dependencies

Intra-repo dependencies may be specified using the `path` attribute, however
Crates.io requires that all dependencies also be published to Crates.io, and be
specified with a version. The path will be used for local development, and the
version will be used for publishing. See [the Crates.io guide][] for more
details.

Intra-repo **dev** dependencies do not require a version specifier.

[the crates.io guide]:
  https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations
