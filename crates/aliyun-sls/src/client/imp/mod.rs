use std::fmt;

#[cfg(feature = "nyquest")]
mod nyquest;
#[cfg(feature = "nyquest")]
pub(crate) use nyquest::*;

#[cfg(feature = "reqwest")]
mod reqwest;
#[cfg(feature = "reqwest")]
pub(crate) use reqwest::*;

impl Response {
    pub fn status(&self) -> StatusCode {
        StatusCode {
            inner: self.inner.status(),
        }
    }

    pub async fn text(self) -> Result<String, Error> {
        self.inner.text().await
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}
