use std::sync::Arc;

use opendal::layers::HttpClientLayer;
use opendal::raw::{HttpBody, HttpClient, HttpFetch};
use opendal::{Buffer, Operator};

#[derive(Clone)]
struct HeaderHttpClient {
    inner: HttpClient,
    headers: Arc<http::HeaderMap>,
}

impl HttpFetch for HeaderHttpClient {
    async fn fetch(
        &self,
        mut req: http::Request<Buffer>,
    ) -> opendal::Result<http::Response<HttpBody>> {
        for (name, value) in self.headers.iter() {
            req.headers_mut().insert(name.clone(), value.clone());
        }
        self.inner.fetch(req).await
    }
}

pub(crate) fn apply_http_headers(
    op: Operator,
    headers: Vec<(String, String)>,
) -> savvy::Result<Operator> {
    if headers.is_empty() {
        return Ok(op);
    }

    if op.info().scheme() != "http" {
        return Err(savvy::Error::new(
            "headers are currently supported only for http/https filesystems",
        ));
    }

    let mut map = http::HeaderMap::new();
    for (name, value) in headers {
        let header_name = http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| savvy::Error::new(&format!("invalid HTTP header name {name:?}: {e}")))?;
        let header_value = http::HeaderValue::from_str(&value).map_err(|e| {
            savvy::Error::new(&format!("invalid value for HTTP header {name:?}: {e}"))
        })?;
        map.insert(header_name, header_value);
    }

    let inner = HttpClient::new()
        .map_err(|e| savvy::Error::new(&format!("cannot create OpenDAL HTTP client: {e}")))?;
    let client = HttpClient::with(HeaderHttpClient {
        inner,
        headers: Arc::new(map),
    });

    Ok(op.layer(HttpClientLayer::new(client)))
}
