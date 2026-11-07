use std::time::Duration;

use futures::channel::oneshot;
use gpui::http_client::{AsyncBody, HttpClient, Request, Response, Result, http};
use http::HeaderValue;

pub struct BlockingHttpClient;

impl HttpClient for BlockingHttpClient {
    fn type_name(&self) -> &'static str {
        "BlockingHttpClient"
    }

    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn send(
        &self,
        request: Request<AsyncBody>,
    ) -> futures::future::BoxFuture<'static, Result<Response<AsyncBody>>> {
        let method = request.method().clone();
        let uri = request.uri().to_string();
        let headers = request.headers().clone();

        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            if method != http::Method::GET {
                let _ = tx.send(Err(anyhow::anyhow!(
                    "BlockingHttpClient only supports GET requests"
                )));
                return;
            }

            let result = (|| -> Result<Response<AsyncBody>> {
                let mut builder = ureq::get(&uri)
                    .config()
                    .timeout_global(Some(Duration::from_secs(20)))
                    .build();
                for (name, value) in &headers {
                    if let Ok(value) = value.to_str() {
                        builder = builder.header(name.as_str(), value);
                    }
                }
                let mut response = builder.call()?;
                let status = response.status();
                let response_headers = response.headers().clone();
                if let Some(length) = response_headers
                    .get(http::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<usize>().ok())
                    && length > 25 * 1024 * 1024
                {
                    return Err(anyhow::anyhow!("image exceeds 25 MiB limit"));
                }
                let body = response
                    .body_mut()
                    .with_config()
                    .limit(25 * 1024 * 1024)
                    .read_to_vec()?;
                let mut builder = http::Response::builder().status(status);
                for (name, value) in &response_headers {
                    builder = builder.header(name, value);
                }
                builder.body(AsyncBody::from(body)).map_err(Into::into)
            })();

            let _ = tx.send(result);
        });

        Box::pin(async move {
            rx.await
                .map_err(|_| anyhow::anyhow!("HTTP worker stopped"))?
        })
    }

    fn proxy(&self) -> Option<&gpui::http_client::Url> {
        None
    }
}
