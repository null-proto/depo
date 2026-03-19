use std::pin::Pin;
use bytes::Bytes;
use http::Response;
use tokio::net::{TcpSocket, TcpStream};
use tower::Service;


#[derive(Debug,Clone,Default)]
pub struct FetchService;


#[allow(unused)]
impl<Request> Service<Request> for FetchService
where 
  Request: ToString + 'static,
{
  type Error = Box<dyn std::error::Error + Send + Sync>;
  type Future = Pin<Box<dyn Future<Output = Result< Self::Response,Self::Error >>>>;
  type Response = Response<Option<Bytes>>;

  fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
    std::task::Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: Request) -> Self::Future {

    Box::pin(async {
      let res = Response::builder()
        .status(200)
        .body(None)
        .map_err(Into::into);


        res
    })
  }
}


