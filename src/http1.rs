#![allow(unused)]

use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

use futures::Stream;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

#[derive(Clone, Debug)]
pub struct Connection<T> {
  stream: T
}

impl<T> Connection<T> {
  pub fn new(io : T) -> Self
  where 
    T: AsyncRead + AsyncWrite + Clone + Unpin
  {
    Self { stream: io }
  }
}

impl<T> Stream for Connection<T> {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
    unimplemented!()
  }
}


pub async fn h1_handler(
  stream: TlsStream<TcpStream>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {


  Ok(())
}

