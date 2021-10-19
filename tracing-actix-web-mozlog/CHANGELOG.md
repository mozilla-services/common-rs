<a name="v0.3.1"></a>

## v0.3.1 (2021-10-19)

### Fixes

- Stop including query strings in `path` field.
  ([64157f4](https://github.com/mozilla-services/common-rs/commit/64157f4574ef8ccd14bc5397d12b3c940aa14654)

<a name="v0.3"></a>

## v0.3 (2021-07-23)

### Breaking Changes

- Update `actix-web` to `=4.0.0-beta.8`.

<a name="v0.2"></a>

## v0.2 (2021-06-08)

### Breaking Changes

- `MozLog` middleware must now be created outside of the `HttpServer::new`
  worker factory closure.
  ([306452e1](https://github.com/mozilla-services/common-rs/commit/306452e1ada47cbe2f0991afd0113289902a8803)

### Features

- Automatically apply the correct Tracing subscriber to handlers.
  ([306452e1](https://github.com/mozilla-services/common-rs/commit/306452e1ada47cbe2f0991afd0113289902a8803)

<a name="v0.1"></a>

## v0.1 (2021-06-02)

Initial release
