# Aliyun SLS Client

Client for [Aliyun SLS](https://help.aliyun.com/zh/sls/) (Aliyun Log Service).

## Feature Flags

### Http backend

- [`reqwest`]:
  `reqwest` feature gate Use [`reqwest`] as the HTTP backend, but do not enable any TLS features.
  - `reqwest-default-tls`: use [`reqwest`] as the HTTP backend and default TLS provider.
  - `reqwest-rustls`: use [`reqwest`] as the HTTP backend and [`rustls`] TLS provider.
  - or, uou can configure the TLS provider by manually enabling feature gates in [`reqwest`].
- [`nyquest`]: A platform native HTTP client, provides smaller binary size.

  To use this, you need to register the http client provider in your application, see
  <https://docs.rs/nyquest-preset/latest/nyquest_preset/#quick-start> for more details:
  - [`nyquest-preset`]: default client configuration for [`nyquest`]
  - [`nyquest-backend-winrt`]: [`UWP/WinRT HttpClient`] for [`nyquest`]
  - [`nyquest-backend-curl`]: libcurl backend for [`nyquest`], requires libcurl _7.68.0_ or later.
  - [`nyquest-backend-nsurlsession`]: macOS/iOS [`NSURLSession`] backend for [`nyquest`].

### Compression

> Note: `lz4` and `deflate` cannot be enabled at the same time.

- `lz4`: enable lz4 compression for logs.
- `deflate`: enable deflate compression for logs.

### Inline Optimizations

Inline features can control how many key-pairs are inlined before spill over to the heap.
If multiple inline features are enabled, the maximum number of inlined key-pairs will be used.

By default, the client will inline up to 8 key-pairs for log messages and 8 key-pairs for log group metadata tags.
 
If `inline-none` feature is enabled, other inline features will be ignored, and no key-pairs will be inlined.

For log message key-pairs, use:
- `inline-keypairs-2`
- `inline-keypairs-4`
- `inline-keypairs-8` (default)
- `inline-keypairs-16`

For log group metadata tags, use:
- `inline-tags-2`
- `inline-tags-4`
- `inline-tags-8` (default)
- `inline-tags-16`

## Unstable Features

> Those features are unstable and requires a nightly build of the Rust toolchain.

- `may_dangle`: This feature makes the Rust compiler less strict about use of vectors that
  contain borrowed references. For details, see the
  [Rustonomicon](https://doc.rust-lang.org/1.42.0/nomicon/dropck.html#an-escape-hatch).

  Tracking issue: [rust-lang/rust#34761](https://github.com/rust-lang/rust/issues/31844)

[`reqwest`]: https://docs.rs/reqwest
[`rustls`]: https://docs.rs/rustls
[`nyquest`]: https://docs.rs/nyquest
[`nyquest-preset`]: https://docs.rs/nyquest-preset
[`nyquest-backend-nsurlsession`]: https://docs.rs/nyquest-backend-nsurlsession
[`UWP/WinRT HttpClient`]: https://learn.microsoft.com/en-us/uwp/api/Windows.Web.Http.HttpClient
[`NSURLSession`]: https://developer.apple.com/documentation/foundation/nsurlsession