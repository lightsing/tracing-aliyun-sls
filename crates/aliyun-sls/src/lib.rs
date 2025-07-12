//! Aliyun SLS Client
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
