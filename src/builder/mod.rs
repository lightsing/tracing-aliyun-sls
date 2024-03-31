use crate::{client, layer, SlsLayer, WorkGuard};
use std::borrow::Cow;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[cfg(feature = "log-comp")]
pub(crate) mod log_comp;

/// A builder for creating a SlsLayer.
#[derive(Clone)]
pub struct SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
    access_key: AccessKey,
    access_secret: AccessSecret,
    endpoint: Endpoint,
    project: Project,
    logstore: Logstore,
    shard_key: Option<Cow<'a, str>>,
    max_level: tracing::Level,
    drain_timeout: std::time::Duration,
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    #[cfg(feature = "deflate")]
    compression_level: u8,
}

impl SlsLayer {
    /// Create a new builder with the required fields and default values.
    pub fn builder() -> SlsTracingBuilder<'static, (), (), (), (), ()> {
        SlsTracingBuilder {
            access_key: (),
            access_secret: (),
            endpoint: (),
            project: (),
            logstore: (),
            shard_key: None,
            max_level: tracing::Level::TRACE,
            drain_timeout: std::time::Duration::from_secs(5),
            #[cfg(feature = "deflate")]
            compression_level: 6,
        }
    }
}

impl<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore>
    SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore>
{
    /// Set the access key.
    pub fn access_key(
        self,
        access_key: impl Into<String>,
    ) -> SlsTracingBuilder<'a, String, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: access_key.into(),
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the access secret.
    pub fn access_secret<'b, S: AsRef<str> + ?Sized>(
        self,
        access_secret: &'b S,
    ) -> SlsTracingBuilder<'a, AccessKey, &'b str, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: access_secret.as_ref(),
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the endpoint.
    pub fn endpoint<'b, S: AsRef<str> + ?Sized>(
        self,
        endpoint: &'b S,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, &'b str, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: endpoint.as_ref(),
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the project.
    pub fn project<'b, S: AsRef<str> + ?Sized>(
        self,
        project: &'b S,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, &'b str, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: project.as_ref(),
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the logstore.
    pub fn logstore<'b, S: AsRef<str> + ?Sized>(
        self,
        logstore: &'b S,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, &'b str> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: logstore.as_ref(),
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the shard key.
    /// If set, the logs will be sent as `KeyHash` mode.
    ///
    /// The `KeyHash` mode is used to send logs to a specific shard.
    /// The key has to be 128 bits hex string.
    ///
    /// Read more about this at:
    /// [PostLogStoreLogs](https://help.aliyun.com/zh/sls/developer-reference/api-postlogstorelogs#section-xit-eeb-tfh)
    pub fn shard_key<S: AsRef<str> + ?Sized>(
        self,
        key: &'a S,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: Some(Cow::Borrowed(key.as_ref())),
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Derive and set the shard key from any string instead of a 128 bits hex string.
    #[cfg_attr(docsrs, doc(cfg(feature = "derive-key")))]
    #[cfg(feature = "derive-key")]
    pub fn derive_shard_key<S: AsRef<str> + ?Sized>(
        self,
        key: &'a S,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: Some(Cow::Owned(
                blake3::hash(key.as_ref().as_bytes()).to_hex().as_str()[..32].to_string(),
            )),
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the maximum level of logs that will be collected.
    ///
    /// Default is `tracing::Level::TRACE`.
    pub fn max_level(
        self,
        level: impl Into<tracing::Level>,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: level.into(),
            drain_timeout: self.drain_timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// How long will the dispatcher wait for more logs before sending logs to SLS.
    ///
    /// Default is 5 seconds.
    pub fn drain_timeout(
        self,
        timeout: std::time::Duration,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: timeout,
            #[cfg(feature = "deflate")]
            compression_level: self.compression_level,
        }
    }

    /// Set the deflate compression level for logs.
    ///
    /// Default is 6.
    ///
    /// As it in `miniz_oxide`, the level can be 0-10:
    /// - NoCompression = 0
    /// - BestSpeed = 1
    /// - BestCompression = 9
    /// - UberCompression = 10
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    #[cfg(feature = "deflate")]
    pub fn compression_level(
        self,
        level: u8,
    ) -> SlsTracingBuilder<'a, AccessKey, AccessSecret, Endpoint, Project, Logstore> {
        SlsTracingBuilder {
            access_key: self.access_key,
            access_secret: self.access_secret,
            endpoint: self.endpoint,
            project: self.project,
            logstore: self.logstore,
            shard_key: self.shard_key,
            max_level: self.max_level,
            drain_timeout: self.drain_timeout,
            compression_level: level,
        }
    }
}

impl SlsTracingBuilder<'_, String, &'_ str, &'_ str, &'_ str, &'_ str> {
    /// Build the SlsLayer and the WorkGuard.
    pub fn build_layer(self) -> (SlsLayer, WorkGuard) {
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
        (
            layer,
            WorkGuard {
                shutdown: Some(shutdown),
            },
        )
    }
}
