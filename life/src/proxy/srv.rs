#![allow(unused)]

// FetchAgent () -> ()
//
// ---
// ClientAgent (req) -> MAP -> (call FetchAgent) -> MAP_POST -> (res)
//
// ---

use std::{
  borrow::Cow,
  error::Error,
  net::SocketAddr,
  path::{Path, PathBuf},
  pin::Pin,
};

use futures::FutureExt;
use life::IntoRequest;
use std::sync::Mutex;
use tokio::{
  io::AsyncWriteExt,
  net::{TcpStream, UnixStream},
  sync::mpsc::{Receiver, Sender},
};
use tracing::event;

use crate::Message;

pub struct FetchAgent {
  sock: UnixStream,
}

pub struct Macher {
  inner: String,
}

pub struct ClientAgent {
  path: PathBuf,
  stream: TcpStream,
  fetch: Option<FetchAgent>,
}

pub struct ProxyAgent {
  path: PathBuf,
  addr: SocketAddr,
  sender: Sender<Message>,
  receiver: Receiver<Message>,
  matcher: Macher,
}

impl FetchAgent {
  pub async fn new(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
    Ok(Self {
      sock: UnixStream::connect(path).await?,
    })
  }

  pub async fn reconnect(&mut self, path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let sock = tokio::select! {
    sock = UnixStream::connect(path) => { sock },
    _ = tokio::time::sleep( std::time::Duration::from_secs(2)) => {
      Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "fetch timeout"))
    }
    }?;
    self.sock = sock;
    Ok(())
  }
}

impl Macher {
  pub fn new<T: Into<String>>(s: T) -> Self {
    Self { inner: s.into() }
  }
}

impl ClientAgent {
  pub fn new(stream: TcpStream, path: PathBuf) -> Self {
    Self {
      fetch: None,
      stream,
      path,
    }
  }
}

impl ProxyAgent {
  pub fn new(
    path: PathBuf,
    addr: SocketAddr,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
  ) -> Self {
    Self {
      path,
      addr,
      sender,
      receiver,
      matcher: Macher::new(""),
    }
  }

  pub async fn runner(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind(&self.addr).await?;
    loop {
      let event = tokio::select! {
        i = self.receiver.recv() => { Ok(i) }
        conn = listener.accept() => { Err(conn) }
      };

      match event {
        Ok(Some(e)) => match e {
          Message::Ok => {
            self.sender.send(Message::Ok).await?;
          }

          Message::Add(s) => {
            self.matcher = Macher::new(s);
            self.sender.send(Message::Ok).await?;
          }

          Message::Del(d) => {
            self.matcher = Macher::new("");
            self.sender.send(Message::Ok).await?;
          }

          _ => {
            self.sender.send(Message::None).await?;
          }
        },

        Err(Ok((stream, peer))) => {
          // new connection opened

        }

        Err(Err(e)) => {
          if let Ok(e) = e.downcast::<std::io::Error>() {
            self
              .sender
              .send(Message::Log(format!("error, {}", e.to_string())))
              .await?;
          }
        }

        Ok(None) => {}
      }
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
    todo!()
  }
}
