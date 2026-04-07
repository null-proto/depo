#![allow(unused)]

// FetchAgent () -> ()
//
// ---
// ClientAgent (req) -> MAP -> (call FetchAgent) -> MAP_POST -> (res)
//

use std::{
  borrow::Cow,
  error::Error,
  net::SocketAddr,
  path::{Path, PathBuf},
  pin::Pin,
};

use futures::FutureExt;
use life::IntoRequest;
use tokio::{io::AsyncWriteExt, net::UnixStream};
use std::sync::Mutex;

pub struct FetchAgent {
  sock:  Mutex<UnixStream>,
}

pub struct Macher {
  inner: String,
}

pub struct ClientAgent {
  path: PathBuf,
  addr: SocketAddr,
  fetch: Option<FetchAgent>,
}

impl FetchAgent {
  pub async fn new(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
    Ok(Self {
      sock: Mutex::new(UnixStream::connect(path).await?),
    })
  }

  pub async fn reconnect(&mut self, path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let sock = tokio::select! {
    sock = UnixStream::connect(path) => { sock },
    _ = tokio::time::sleep( std::time::Duration::from_secs(2)) => {
      Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Fetch timeout"))
    }
    }?;

    self.sock.lock().and_then(move |mut i| {
      *i = sock;
      Ok(())
    });

    Ok(())
  }
}

impl Macher {
  pub fn new<T: Into<String>>(s: T) -> Self {
    Self { inner: s.into() }
  }
}

impl ClientAgent {
  pub fn new(path: PathBuf, addr: SocketAddr) -> Self {
    Self {
      path,
      addr,
      fetch: None,
    }
  }
}

impl<R> tower::Service<R> for FetchAgent
where
  R: IntoRequest + 'static,
{
  type Error = Box<dyn Error + Send + Sync>;
  type Response = life::Response;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

  fn poll_ready(
    &mut self,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Result<(), Self::Error>> {
    std::task::Poll::Pending
  }

  fn call(&mut self, req: R) -> Self::Future {
    let req = req.into_req();
    let mut sock = self.sock.lock().unwrap();

    Box::pin(async move {
      sock.write(&req.into_vec_u8()).await?;
      // sock.flush().await?;
    //   let res = life::Response::read_from(*sock).await?;
    //   Ok(res)
      todo!()
    })
  }
}
