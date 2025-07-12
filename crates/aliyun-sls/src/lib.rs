//! Client for [Aliyun SLS](https://help.aliyun.com/zh/sls/) (Aliyun Log Service).
//!
//! ## Feature Flags
//!
//! ### Http backend
//!
//! - [`reqwest`]:
//!   `reqwest` feature gate Use [`reqwest`] as the HTTP backend, but do not enable any TLS features.
//!   - `reqwest-default-tls`: use [`reqwest`] as the HTTP backend and default TLS provider.
//!   - `reqwest-rustls`: use [`reqwest`] as the HTTP backend and [`rustls`] TLS provider.
//!   - or, uou can configure the TLS provider by manually enabling feature gates in [`reqwest`].
//! - [`nyquest`]: A platform native HTTP client, provides smaller binary size.
//!
//!   To use this, you need to register the http client provider in your application, see
//!   <https://docs.rs/nyquest-preset/latest/nyquest_preset/#quick-start> for more details:
//!   - [`nyquest-preset`]: default client configuration for [`nyquest`]
//!   - [`nyquest-backend-winrt`]: [`UWP/WinRT HttpClient`] for [`nyquest`]
//!   - [`nyquest-backend-curl`]: libcurl backend for [`nyquest`], requires libcurl _7.68.0_ or later.
//!   - [`nyquest-backend-nsurlsession`]: macOS/iOS [`NSURLSession`] backend for [`nyquest`].
//!
//! ### Compression
//!
//! > Note: `lz4` and `deflate` cannot be enabled at the same time.
//!
//! - `lz4`: enable lz4 compression for logs.
//! - `deflate`: enable deflate compression for logs.
//!
//! [`reqwest`]: https://docs.rs/reqwest
//! [`rustls`]: https://docs.rs/rustls
//! [`nyquest`]: https://docs.rs/nyquest
//! [`nyquest-preset`]: https://docs.rs/nyquest-preset
//!
//! [`nyquest-backend-nsurlsession`]: https://docs.rs/nyquest-backend-nsurlsession
//! [`UWP/WinRT HttpClient`]: https://learn.microsoft.com/en-us/uwp/api/Windows.Web.Http.HttpClient
//! [`NSURLSession`]: https://developer.apple.com/documentation/foundation/nsurlsession
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(all(feature = "lz4", feature = "deflate"), not(docsrs)))]
compile_error!("`lz4` and `deflate` cannot be enabled at the same time");

mod client;
mod proto;

pub use client::{SlsClient, SlsClientBuilder, SlsClientBuilderError, SlsClientError};
pub use proto::{Log, LogGroupMetadata};

#[cfg(test)]
#[cfg_attr(test, ctor::ctor)]
fn init() {
    // Initialize the tracing subscriber for tests
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .init();
}
