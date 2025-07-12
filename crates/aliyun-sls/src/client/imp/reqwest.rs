use crate::client::headers;
use async_lock::OnceCell;
use http::HeaderMap;
use reqwest::header::{HeaderName, HeaderValue};

static HTTP_CLIENT: OnceCell<HttpClient> = OnceCell::new();

#[derive(Clone)]
pub(crate) struct HttpClient {
    inner: reqwest::Client,
}

#[must_use = "RequestBuilder does nothing until you 'send' it"]
pub(crate) struct RequestBuilder {
    inner: reqwest::RequestBuilder,
}

pub(crate) struct Response {
    pub(crate) inner: reqwest::Response,
}

pub(crate) struct StatusCode {
    pub(crate) inner: http::StatusCode,
}

pub type Error = reqwest::Error;
type Result<T, E = Error> = std::result::Result<T, E>;

impl HttpClient {
    async fn new() -> Result<Self> {
        Ok(Self {
            inner: reqwest::ClientBuilder::new()
                .user_agent(headers::USER_AGENT_VALUE)
                .https_only(true)
                .default_headers(HeaderMap::from_iter([
                    (
                        HeaderName::from_static(headers::CONTENT_TYPE),
                        HeaderValue::from_static(headers::DEFAULT_CONTENT_TYPE),
                    ),
                    (
                        HeaderName::from_static(headers::LOG_API_VERSION),
                        HeaderValue::from_static(headers::API_VERSION),
                    ),
                    (
                        HeaderName::from_static(headers::LOG_SIGNATURE_METHOD),
                        HeaderValue::from_static(headers::SIGNATURE_METHOD),
                    ),
                ]))
                .build()?,
        })
    }

    pub async fn get_or_try_init() -> Result<&'static Self> {
        HTTP_CLIENT.get_or_try_init(HttpClient::new).await
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            inner: self.inner.post(url),
        }
    }
}

impl RequestBuilder {
    pub fn header<K, V>(self, key: K, value: V) -> RequestBuilder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        RequestBuilder {
            inner: self.inner.header(key, value),
        }
    }

    pub fn body(self, body: Vec<u8>) -> RequestBuilder {
        RequestBuilder {
            inner: self.inner.body(body),
        }
    }

    pub async fn send(self) -> Result<Response> {
        Ok(Response {
            inner: self.inner.send().await?.error_for_status()?,
        })
    }
}
