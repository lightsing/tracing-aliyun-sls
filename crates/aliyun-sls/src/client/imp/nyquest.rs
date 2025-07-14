use crate::client::headers;
use async_lock::OnceCell;
use std::borrow::Cow;

static HTTP_CLIENT: OnceCell<HttpClient> = OnceCell::new();

#[derive(Clone)]
pub(crate) struct HttpClient {
    inner: nyquest::AsyncClient,
}

#[must_use = "RequestBuilder does nothing until you 'send' it"]
pub(crate) struct RequestBuilder {
    client: HttpClient,
    inner: nyquest::Request<nyquest_interface::r#async::BoxedStream>,
}

pub(crate) struct Response {
    pub(crate) inner: nyquest::r#async::Response,
}

pub(crate) struct StatusCode {
    pub(crate) inner: nyquest::StatusCode,
}

pub type Error = nyquest::Error;
type Result<T, E = Error> = std::result::Result<T, E>;

impl HttpClient {
    async fn new() -> Result<Self> {
        Ok(Self {
            inner: nyquest::ClientBuilder::default()
                .user_agent(headers::USER_AGENT_VALUE)
                // .with_header(headers::CONTENT_TYPE, headers::DEFAULT_CONTENT_TYPE)
                .with_header(headers::LOG_API_VERSION, headers::API_VERSION)
                .with_header(headers::LOG_SIGNATURE_METHOD, headers::SIGNATURE_METHOD)
                .build_async()
                .await?,
        })
    }

    pub async fn get_or_try_init() -> Result<&'static Self> {
        HTTP_CLIENT.get_or_try_init(HttpClient::new).await
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            client: self.clone(),
            inner: nyquest::Request::post(url.to_string()),
        }
    }
}

impl RequestBuilder {
    pub fn header(
        self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> RequestBuilder {
        RequestBuilder {
            client: self.client,
            inner: self.inner.with_header(key, value),
        }
    }

    pub fn body(self, body: Vec<u8>) -> RequestBuilder {
        RequestBuilder {
            client: self.client,
            inner: self
                .inner
                .with_body(nyquest::Body::bytes(body, headers::DEFAULT_CONTENT_TYPE)),
        }
    }

    pub async fn send(self) -> Result<Response> {
        let res = self.client.inner.request(self.inner).await?;
        Ok(Response { inner: res })
    }
}

impl StatusCode {
    pub(crate) fn is_success(&self) -> bool {
        self.inner.is_successful()
    }
}

impl From<StatusCode> for u16 {
    fn from(status: StatusCode) -> u16 {
        status.inner.code()
    }
}
