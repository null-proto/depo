use std::convert::Infallible;

use bytes::Bytes;
use http::{Request, Response};
use hyper::body::Incoming;

use crate::body::HttpBody;

pub async fn proxy_run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let listener = tokio::net::TcpListener::bind("127.0.0.2:8080").await?;

  loop {
    if let Ok((stream, _peer)) = listener.accept().await {
      match tokio::spawn(async move {
        let io = hyper_util::rt::TokioIo::new(stream);

        if let Err(err) = hyper::server::conn::http1::Builder::new()
          .serve_connection(io, hyper::service::service_fn(proxy_service))
          .await
        {
          tracing::error!("{}", err);
        }
      })
      .await
      {
        Err(err) => {
          tracing::error!("thread failed: {}", err);
        }
        _ => {}
      }
    }
  }
}

pub async fn proxy_service(
  req: Request<Incoming>,
) -> Result<Response<HttpBody<Bytes>>, Infallible> {
  tracing::info!("web has being served {}", req.uri());

  Ok(http::Response::new(HttpBody::new(Bytes::from("hello"))))
}
