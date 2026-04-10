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
  fmt::Pointer,
  net::SocketAddr,
  ops::Deref,
  path::{Path, PathBuf},
  pin::Pin,
};

use futures::FutureExt;
use life::{Frame, IntoRequest, Request, Response};
use std::sync::Mutex;
use tokio::{
  io::AsyncWriteExt,
  net::{TcpStream, UnixStream},
  sync::{
    mpsc::{Receiver, Sender},
    watch::Receiver as Watcher,
  },
};
use tracing::event;

use crate::Message;

pub struct FetchAgent {
  path: PathBuf,
  sock: Option<UnixStream>,
}

#[derive(Clone)]
pub struct Macher {
  pub inner: String,
}

pub struct ClientAgent {
  stream: TcpStream,
  sender: Sender<Message>,
  watcher: Watcher<Macher>,
  fetch: FetchAgent,
}

pub struct ProxyAgent {
  path: PathBuf,
  addr: SocketAddr,
  sender: Sender<Message>,
  receiver: Receiver<Message>,
  watcher: Watcher<Macher>,
}

impl FetchAgent {
  pub fn new(path: PathBuf) -> Self {
    Self { sock: None, path }
  }

  pub async fn reconnect(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut sock = tokio::select! {
    sock = UnixStream::connect(&self.path) => { sock },
    _ = tokio::time::sleep( std::time::Duration::from_secs(2)) => {
      Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "fetch timeout"))
    }
    }?;
    self.sock = Some(sock);
    Ok(())
  }
}

impl Macher {
  pub fn new<T: Into<String>>(s: T) -> Self {
    Self { inner: s.into() }
  }
}

impl ClientAgent {
  pub fn new(
    stream: TcpStream,
    path: PathBuf,
    sender: Sender<Message>,
    watcher: Watcher<Macher>,
  ) -> Self {
    Self {
      fetch: FetchAgent::new(path),
      stream,
      sender,
      watcher,
    }
  }
}

impl ProxyAgent {
  pub fn new(
    path: PathBuf,
    addr: SocketAddr,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    watcher: Watcher<Macher>,
  ) -> Self {
    Self {
      path,
      addr,
      sender,
      receiver,
      watcher,
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

          _ => {
            self.sender.send(Message::None).await?;
          }
        },

        Err(Ok((stream, peer))) => {
          // new connection opened

          let mut client = ClientAgent::new(
            stream,
            self.path.clone(),
            self.sender.clone(),
            self.watcher.clone(),
          );

          tokio::spawn(async move {
            if let Err(e) = client.call().await {
              client
                .stream
                .write(&Frame::new_err("cannot connect".to_string()).into_vec_u8())
                .await;
              client
                .sender
                .send(Message::Log(format!("connection dropped due to {}", e)))
                .await;
            }
          });
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
      };
    }
  }
}

impl FetchAgent {
  async fn call<R: IntoRequest + 'static>(
    &mut self,
    req: R,
  ) -> Result<Response, Box<dyn Error + Send + Sync>> {
    let req = req.into_req();
    let mut retry = 0u8;

    while retry < 3 {
      if let Some(sock) = &mut self.sock {
        if sock.write(&req.into_vec_u8()).await.is_ok() {
          return Response::read_from(sock).await;
        };
      } else {
        self.reconnect().await?;
        retry += 1;
      }
    }

    Err(
      Box::new(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "retry exceeded",
      ))
      .into(),
    )
  }
}

impl ClientAgent {
  async fn call(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut m = self.watcher.borrow_and_update().deref().clone();

    loop {
      let creq = Request::read_from(&mut self.stream).await?;

      if self.watcher.has_changed()? {
        self.sender.send(Message::Ok).await?;
        m = self.watcher.borrow_and_update().deref().clone();
      }

      if !m.inner.is_empty() {
        if let life::Frame::Res(s) = &creq.frame {
          if s.starts_with(&m.inner) {
            let (tx, rx) = tokio::sync::oneshot::channel::<Message>();
            self
              .sender
              .send(Message::Blocked(crate::Block {
                sender: tx,
                matcher: s.clone(),
              }))
              .await?;
            match rx.await? {
              Message::Kill => {
                self
                  .stream
                  .write(Frame::Reset.into_res().into_vec_u8().as_slice())
                  .await?;
                self.stream.flush().await?;
                continue;
              }
              _ => {}
            }
          }
        }
      }

      let sres = self.fetch.call(creq).await?;
      self.stream.write(sres.into_vec_u8().as_slice()).await?;
      self.stream.flush().await?;

      if sres.is_err() {
        break Ok(());
      }
    }
  }
}
