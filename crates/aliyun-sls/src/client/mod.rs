//! Aliyun SLS client

pub use self::builder::{SlsClientBuilder, SlsClientBuilderError};
use crate::{
    Log, LogGroupMetadata,
    proto::{calc_log_group_encoded_len, encode_log_group},
};
use std::sync::Arc;
use tracing::{Instrument, Level};

mod builder;
mod headers;
mod imp;
mod signer;

/// A client for sending logs to Aliyun SLS (Simple Log Service).
#[derive(Clone)]
pub struct SlsClient {
    inner: Arc<SlsClientInner>,
}

struct SlsClientInner {
    url: String,
    signer: signer::Signer,
    enable_trace: bool,
    print_internal_error: bool,
    #[cfg(feature = "deflate")]
    compression_level: u8,
}

/// Error type for SLS client operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SlsClientError {
    /// Non-successful HTTP response from the SLS service.
    #[error("http error [{status}] {message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Error message from the response.
        message: Box<str>,
    },
    /// Other HTTP client error.
    #[error("other http client error: {0}")]
    Imp(#[from] imp::Error),
}

impl SlsClient {
    /// Create a new SLS client builder.
    pub fn builder() -> SlsClientBuilder<'static> {
        SlsClientBuilder::default()
    }

    /// Put a log group to Aliyun SLS.
    pub async fn put_log(&self, metadata: &LogGroupMetadata, logs: &[Log]) {
        self.try_put_log(metadata, logs).await.ok();
    }

    /// Try to put a log group to Aliyun SLS.
    pub async fn try_put_log(
        &self,
        metadata: &LogGroupMetadata,
        logs: &[Log],
    ) -> Result<(), SlsClientError> {
        let fut = async move {
            return match self.put_log_inner(metadata, logs).await {
                Err(e) => {
                    if self.inner.enable_trace {
                        tracing::error!(err = ?e);
                    } else if self.inner.print_internal_error {
                        eprintln!("[tracing-aliyun-sls] error putting log: {e}");
                    }
                    Err(e)
                }
                Ok(()) => Ok(()),
            };
        };
        if self.inner.enable_trace {
            fut.instrument(tracing::span!(Level::TRACE, "put_log", target = %self.inner.signer.canonicalized_resource)).await
        } else {
            fut.await
        }
    }

    async fn put_log_inner(
        &self,
        metadata: &LogGroupMetadata,
        logs: &[Log],
    ) -> Result<(), SlsClientError> {
        let http_client = imp::HttpClient::get_or_try_init().await?;

        let raw_length = calc_log_group_encoded_len(metadata, logs);
        let mut buf = Vec::with_capacity(raw_length);
        encode_log_group(&mut buf, metadata, logs).expect("infallible");
        #[cfg(feature = "lz4")]
        let buf = lz4_flex::compress(&buf);
        #[cfg(feature = "deflate")]
        let buf = miniz_oxide::deflate::compress_to_vec_zlib(&buf, self.inner.compression_level);

        let signature = self.inner.signer.sign(raw_length, &buf);
        let builder = http_client
            .post(&self.inner.url)
            .header(headers::AUTHORIZATION, signature.authorization)
            .header(headers::CONTENT_LENGTH, buf.len().to_string())
            .header(headers::CONTENT_MD5, signature.content_md5)
            .header(headers::DATE, signature.date)
            .header(headers::LOG_BODY_RAW_SIZE, signature.raw_length);

        #[cfg(feature = "lz4")]
        let builder = builder.header(headers::LOG_COMPRESS_TYPE, "lz4");
        #[cfg(feature = "deflate")]
        let builder = builder.header(headers::LOG_COMPRESS_TYPE, "deflate");

        let res = builder.body(buf).send().await?;
        if self.inner.enable_trace {
            let status = res.status();
            let res = res.text().await?;
            tracing::trace!(%status, %res);
            if !status.is_success() {
                return Err(SlsClientError::Http {
                    status: status.into(),
                    message: res.into_boxed_str(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::client::SlsClientBuilder;

    #[tokio::test]
    async fn test() {
        use crate::proto::*;

        let builder = SlsClientBuilder::default()
            .access_key(env!("ACCESS_KEY"))
            .access_secret(env!("ACCESS_SECRET"))
            .unwrap()
            .endpoint("cn-guangzhou.log.aliyuncs.com")
            .project("playground")
            .logstore("test")
            .enable_trace(true);

        #[cfg(feature = "deflate")]
        let builder = builder.compression_level(10);

        let client = builder.build().unwrap();

        let metadata =
            LogGroupMetadata::default().with_tag(MayStaticKey::from_static("static"), "test");
        let logs = vec![Log::default().with(MayStaticKey::from_static("message"), "hello world")];

        client.put_log(&metadata, &logs).await;
    }
}
