use crate::proto::{LogGroup, Message};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use headers::*;
use hmac::{Hmac, Mac};
use reqwest::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{header::DATE, Client, ClientBuilder};
use sha1::Sha1;
use std::sync::Arc;

#[allow(dead_code)]
mod headers {
    pub const API_VERSION: &str = "0.6.0";
    pub const SIGNATURE_METHOD: &str = "hmac-sha1";
    pub const DEFAULT_CONTENT_TYPE: &str = "application/x-protobuf";
    pub const USER_AGENT_VALUE: &str =
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
    pub const LOG_API_VERSION: &str = "x-log-apiversion";
    pub const LOG_SIGNATURE_METHOD: &str = "x-log-signaturemethod";
    pub const LOG_BODY_RAW_SIZE: &str = "x-log-bodyrawsize";
    pub const LOG_COMPRESS_TYPE: &str = "x-log-compresstype";
    pub const CONTENT_MD5: &str = "Content-MD5";
}

#[derive(Clone)]
pub struct SlsClient {
    inner: Arc<Inner>,
}

struct Inner {
    access_key: String,
    url: String,
    canonicalized_resource: String,
    client: Client,
    hmac: Hmac<Sha1>,
    #[cfg(feature = "deflate")]
    compression_level: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum SlsClientError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Invalid Key: {0}")]
    Hmac(#[from] hmac::digest::InvalidLength),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl SlsClient {
    pub fn new(
        access_key: String,
        access_secret: impl AsRef<str>,
        endpoint: impl AsRef<str>,
        project: impl AsRef<str>,
        logstore: impl AsRef<str>,
        shard_key: Option<impl AsRef<str>>,
        #[cfg(feature = "deflate")] compression_level: u8,
    ) -> Result<Self, SlsClientError> {
        let canonicalized_resource = match shard_key {
            None => format!("/logstores/{}/shards/lb", logstore.as_ref()),
            Some(key) => format!(
                "/logstores/{}/shards/route?key={}",
                logstore.as_ref(),
                key.as_ref()
            ),
        };
        let url = format!(
            "https://{}.{}{}",
            project.as_ref(),
            endpoint.as_ref(),
            &canonicalized_resource
        );
        let client = ClientBuilder::new()
            .user_agent(USER_AGENT_VALUE)
            .https_only(true)
            .build()?;
        let hmac = Hmac::<Sha1>::new_from_slice(access_secret.as_ref().as_bytes())?;
        let inner = Inner {
            access_key,
            url,
            canonicalized_resource,
            client,
            hmac,
            #[cfg(feature = "deflate")]
            compression_level,
        };
        let inner = Arc::new(inner);
        Ok(Self { inner })
    }

    /// SignString = VERB + "\n"
    ///              + CONTENT-MD5 + "\n"
    ///              + CONTENT-TYPE + "\n"
    ///              + DATE + "\n"
    ///              + CanonicalizedLOGHeaders + "\n"
    ///              + CanonicalizedResource
    pub async fn put_log(&self, log: &LogGroup<'_>) -> Result<(), SlsClientError> {
        let mut mac = self.inner.hmac.clone();

        mac.update(b"POST\n");

        let mut buf = Vec::with_capacity(log.encoded_len());
        log.encode(&mut buf)?;
        #[cfg(feature = "lz4")]
        let buf = lz4_flex::compress(&buf);
        #[cfg(feature = "deflate")]
        let buf = miniz_oxide::deflate::compress_to_vec_zlib(&buf, self.inner.compression_level);
        let content_md5 = hex::encode_upper(md5::compute(buf.as_slice()).as_ref());
        mac.update(content_md5.as_bytes());
        mac.update(b"\n");

        mac.update(DEFAULT_CONTENT_TYPE.as_bytes());
        mac.update(b"\n");

        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        mac.update(date.as_bytes());
        mac.update(b"\n");

        // CanonicalizedLOGHeaders的构造方式如下：
        //     将所有以x-log和x-acs为前缀的HTTP请求头的名字转换成小写字母。
        //     将上一步得到的所有LOG自定义请求头按照字典顺序进行升序排序。
        //     删除请求头和内容之间分隔符两端出现的任何空格。
        //     将所有的头和内容用\n分隔符组合成最后的CanonicalizedLOGHeader。
        mac.update(LOG_API_VERSION.as_bytes());
        mac.update(b":");
        mac.update(API_VERSION.as_bytes());
        mac.update(b"\n");
        mac.update(LOG_BODY_RAW_SIZE.as_bytes());
        mac.update(b":");
        mac.update(log.encoded_len().to_string().as_bytes());
        #[cfg(not(any(feature = "lz4", feature = "deflate")))]
        mac.update(b"\n");
        #[cfg(feature = "lz4")]
        mac.update(b"\nx-log-compresstype:lz4\n");
        #[cfg(feature = "deflate")]
        mac.update(b"\nx-log-compresstype:deflate\n");
        mac.update(LOG_SIGNATURE_METHOD.as_bytes());
        mac.update(b":");
        mac.update(SIGNATURE_METHOD.as_bytes());
        mac.update(b"\n");

        // CanonicalizedResource的构造方式如下：
        // a. 将CanonicalizedResource设置为空字符串" "。
        // b. 放入要访问的LOG资源，如/logstores/logstorename（如果没有logstorename则可不填写）。
        // c. 如果请求包含查询字符串QUERY_STRING，则在CanonicalizedResource字符串尾部添加?和查询字符串。
        //
        // QUERY_STRING是URL中请求参数按字典顺序排序后的字符串，其中参数名和值之间用=相隔组成字符串，并对参数名-值对按照字典顺序升序排序，然后以&符号连接构成字符串。其公式化描述如下：
        // QUERY_STRING = "KEY1=VALUE1" + "&" + "KEY2=VALUE2"
        mac.update(self.inner.canonicalized_resource.as_bytes());
        let authorization = BASE64_STANDARD.encode(mac.finalize().into_bytes());
        let authorization = format!("LOG {}:{}", self.inner.access_key, authorization);
        let builder = self
            .inner
            .client
            .post(&self.inner.url)
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, DEFAULT_CONTENT_TYPE)
            .header(CONTENT_LENGTH, buf.len())
            .header(CONTENT_MD5, content_md5)
            .header(DATE, date)
            .header(LOG_API_VERSION, API_VERSION)
            .header(LOG_BODY_RAW_SIZE, log.encoded_len())
            .header(LOG_SIGNATURE_METHOD, SIGNATURE_METHOD);
        #[cfg(feature = "lz4")]
        let builder = builder.header(LOG_COMPRESS_TYPE, "lz4");
        #[cfg(feature = "deflate")]
        let builder = builder.header(LOG_COMPRESS_TYPE, "deflate");
        let res = builder.body(buf).send().await?;

        // we can not produce logs if the request fails,
        // otherwise the log itself will be logged
        if !res.status().is_success() {
            eprintln!(
                "Failed to send log to sls: status_code={}, error={}",
                res.status(),
                res.text().await?
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::client::SlsClient;
    use crate::SlsLayer;
    use chrono::Utc;

    #[tokio::test]
    async fn test() {
        use crate::proto::*;

        SlsLayer::builder().access_secret(&"a".to_string());

        let client = SlsClient::new(
            env!("ACCESS_KEY").to_string(),
            env!("ACCESS_SECRET"),
            "cn-guangzhou.log.aliyuncs.com",
            "playground",
            "test",
            Some("00000000000000000000000000000000"),
            #[cfg(feature = "deflate")]
            10,
        )
        .unwrap();

        let log_group = LogGroup {
            logs: vec![Log {
                time: Utc::now().timestamp() as u32,
                contents: vec![KeyValue {
                    key: "message",
                    value: "hello world".into(),
                }],
                time_ns: None,
            }],
            reserved: None,
            topic: None,
            source: None,
            log_tags: vec![],
        };

        client.put_log(&log_group).await.unwrap();
    }
}
