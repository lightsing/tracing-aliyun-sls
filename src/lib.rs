//! A tracing layer that sends logs to Aliyun SLS.
//!
//! ## Feature Flags
//!
//! Note: `lz4` and `deflate` cannot be enabled at the same time.
//!
//! - `lz4`: enable lz4 compression for logs.
//! - `deflate`: enable deflate compression for logs.
//!
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(all(feature = "lz4", feature = "deflate"), not(docsrs)))]
compile_error!("`lz4` and `deflate` cannot be enabled at the same time");

use std::borrow::Cow;
use std::collections::HashMap;
use tokio::sync::mpsc;

mod client;
mod layer;
mod proto;

pub use crate::layer::{SlsLayer, WorkGuard};

/// A builder for creating a SlsLayer.
pub struct SlsTracingBuilder<'a> {
    access_key: String,
    access_secret: Cow<'a, str>,
    endpoint: Cow<'a, str>,
    project: Cow<'a, str>,
    logstore: Cow<'a, str>,
    shard_key: Option<Cow<'a, str>>,
    max_level: tracing::Level,
    drain_timeout: std::time::Duration,
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    #[cfg(feature = "deflate")]
    compression_level: u8,
}

impl<'a> SlsTracingBuilder<'a> {
    /// Create a new builder with the required fields.
    pub fn new(
        access_key: impl Into<String>,
        access_secret: &'a str,
        endpoint: &'a str,
        project: &'a str,
        logstore: &'a str,
    ) -> Self {
        Self {
            access_key: access_key.into(),
            access_secret: Cow::Borrowed(access_secret),
            endpoint: Cow::Borrowed(endpoint),
            project: Cow::Borrowed(project),
            logstore: Cow::Borrowed(logstore),
            shard_key: None,
            max_level: tracing::Level::TRACE,
            drain_timeout: std::time::Duration::from_secs(5),
            #[cfg(feature = "deflate")]
            compression_level: 6,
        }
    }

    /// If set, the logs will be sent as `KeyHash` mode.
    ///
    /// The `KeyHash` mode is used to send logs to a specific shard.
    ///
    /// Read more about this at:
    /// [PostLogStoreLogs](https://help.aliyun.com/zh/sls/developer-reference/api-postlogstorelogs#section-xit-eeb-tfh)
    pub fn shard_key(mut self, key: &'a str) -> Self {
        self.shard_key = Some(Cow::Borrowed(key));
        self
    }

    /// Set the maximum level of logs that will be collected.
    pub fn max_level(mut self, level: impl Into<tracing::Level>) -> Self {
        self.max_level = level.into();
        self
    }

    /// How long will the dispatcher wait for more logs before sending logs to SLS.
    pub fn drain_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.drain_timeout = timeout;
        self
    }

    /// Set the deflate compression level for logs.
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    #[cfg(feature = "deflate")]
    pub fn compression_level(mut self, level: u8) -> Self {
        self.compression_level = level;
        self
    }

    /// Build the SlsLayer and the WorkGuard.
    pub fn layer(self) -> (SlsLayer, WorkGuard) {
        let (sender, receiver) = mpsc::channel(1024);
        let (shutdown, shutdown_rx) = mpsc::channel(1);
        let layer = SlsLayer {
            max_level: self.max_level,
            sender,
        };
        let mut dispatcher = layer::SlsDispatcher {
            receiver,
            client: client::SlsClient::new(
                self.access_key,
                self.access_secret,
                self.endpoint,
                self.project,
                self.logstore,
                self.shard_key,
                #[cfg(feature = "deflate")]
                self.compression_level,
            )
            .unwrap(),
            buffer: HashMap::new(),
            drain_timeout: self.drain_timeout,
            shutdown: shutdown_rx,
        };
        tokio::spawn(async move { dispatcher.run().await });
        (layer, WorkGuard { shutdown })
    }
}
