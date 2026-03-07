use std::pin::Pin;

use futures::SinkExt;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use futures::Stream;
use futures::StreamExt;
use futures::Sink;
use tower::Service;
use tracing as log;

#[derive( Debug)]
pub struct Connection<T> {
  stream: T,
  writebuf : Vec<u8>
}

impl<T> Connection<T> {
  pub fn new(io: T) -> Self
  where
    T: AsyncRead + AsyncWrite  + Unpin,
  {
    Self { stream: io , writebuf: vec![] }
  }
}

impl<T: AsyncRead + Unpin> Stream for Connection<T> {
  type Item = Vec<u8>;

  fn poll_next(
    self: Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {

    let mut buf_ = vec![0u8;8000];
    let mut buffer = tokio::io::ReadBuf::new(buf_.as_mut_slice());
    let s = Pin::new(&mut self.get_mut().stream);
    s.poll_read(cx,&mut buffer)
      .map(|_| Some(buf_) )
  }
}

impl<T: AsyncWrite + Unpin, Item> Sink<Item> for Connection<T> 
where 
  Item: Into<Vec<u8>>
{
  type Error = Box<dyn std::error::Error + Send + Sync>;

  fn poll_ready(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
    Pin::new(&mut self.get_mut().stream)
    .poll_write(cx, &[])
    .map_ok(|_| () )
    .map_err(|e| e.into())
  }

  fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
    let item = item.into();
    let mut _buf = &mut self.get_mut().writebuf;
    _buf.extend(item);
    Ok(())
  }

  fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
    let s = &mut self.get_mut();

    let _buf : Vec<u8> = std::mem::take(&mut s.writebuf);

    _ = Pin::new(&mut s.stream)
      .poll_write(cx, &_buf)
      .map_err(|e| Box::new(e) as Self::Error )?;

    Pin::new(&mut s.stream)
      .poll_flush(cx)
      .map_err(|e| e.into())
  }

  fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
    Pin::new(&mut self.get_mut().stream)
      .poll_shutdown(cx)
      .map_err(|e| e.into())
  }
}

impl<T, Request> Service<Request> for Connection<T>
where
  Request: Into<Vec<u8>> + 'static,
  T: AsyncRead + AsyncWrite + Unpin
{
  type Error = Box<dyn std::error::Error + Send + Sync>;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;
  type Response = Vec<u8>;

  fn poll_ready(
    &mut self,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Result<(), Self::Error>> {
    Pin::new(&mut self.stream)
    .poll_write(cx, &[])
    .map_ok(|_| () )
    .map_err(|e| e.into())

    // std::task::Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: Request) -> Self::Future {
    let req : Vec<u8> = req.into();
    Box::pin(async move {
      log::trace!("ingres: {}", String::from_utf8_lossy(req.as_slice()));
      Ok(req)
    })
  }
}

impl<T> Connection<T>
where 
  T: AsyncRead + AsyncWrite + Unpin
{
  pub async  fn handler(&mut self) -> Result<() , Box<dyn std::error::Error + Send + Sync>> {
    if let Some(req) = self.next().await {

      let _a = self.call(req).await?;

      // tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
      self.send(b"HTTP/1.1 200 Ok\r\nConnection: Close\r\nContent-Length: 5\r\n\r\nhello").await?;
    } else {
      log::debug!("request dropped");
    }
    Ok(())
  }
}

pub async fn h1_handler<T : AsyncRead + AsyncWrite + Unpin>(
  stream: T,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  // one conn = one req/res = one use
  let mut conn = Connection::new(stream);

  conn.handler().await?;

  Ok(())
}
