use std::io::IoSlice;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

// #[tokio::main(flavor = "multi_thread")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");
  let s_path = path.clone();

  let s = tokio::spawn(async move {
    server(&s_path).await;
  });

  let c = tokio::spawn(async move {
    client(&path).await;
  });

  _ = tokio::join!(s, c);
}

async fn server(path: &std::path::Path) {
  let listener = tokio::net::UnixListener::bind(path).unwrap();

  tracing::info!(target: "server","server initiated: listeng at {:?}", path);
  loop {
    let stream = listener.accept().await;
    tokio::spawn(async move {
      if let Ok((mut stream, addr)) = stream {
        tracing::info!(target: "server","connect established: {:?}", addr);

        // let (mut read, write) = stream.into_split();
        match server_heldler(&mut stream).await {
          Ok(_) => {}
          Err(_) => {}
        }

        // let read_thread = tokio::spawn(async move {
        //   tracing::debug!(target: "server","connect established: setup reader");
        //   loop {
        //     match server_read_handler(&mut read).await {
        //       Err(err) => {
        //         if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
        //           tracing::error!( target: "server", "c: {:?} {}", addr, err);
        //         }
        //       }
        //
        //       _ => {
        //         tracing::warn!(target: "server", "connection satisfied: {addr:?}");
        //       }
        //     }
        //   }
        // });
        //
        // let write_thread = tokio::spawn(async move {
        //   let _w = write;
        //   tracing::debug!(target: "server","connect established: setup writer");
        // });
        //
        // _ = tokio::join!(read_thread, write_thread);
      } else {
        tracing::error!("connect failed to establish: hard reset")
      }
    });
  }
}

async fn server_heldler(
  mut stream: &mut tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  loop {
    match server_read_handler(&mut stream).await {
      Err(err) => {
        if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
          tracing::error!( target: "server", "c: {:?} {}", stream.peer_addr(), err);
        }
      }

      _ => {
        tracing::warn!(target: "server", "connection satisfied: {:?}", stream.peer_addr());
      }
    }
  }
}

#[allow(unreachable_code)]
async fn server_read_handler(
  stream: &mut tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let mut buf = Box::pin(tokio::io::BufReader::new(stream));
  let mut sbuf = String::new();

  loop {
    buf.read_line(&mut sbuf).await?;

    tracing::info!(target: "server" , "ingress: {:?}", sbuf.trim_end());
    sbuf.truncate(0);
  }

  Ok(())
}

#[allow(unreachable_code)]
async fn client_write_handler(
  mut conn: tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing::info!(target :"client" , "connection established {conn:?}");
  let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

  conn.write(b"client hello\n").await?;
  conn.flush().await?;

  let (read, mut write) = conn.into_split();

  let writer = tokio::spawn(async move {
    let mut s2 = String::new();
    loop {
      stdin.read_line(&mut s2).await.unwrap();

      if !s2.is_empty() {
        if s2 == "exit" {
          std::process::exit(0);
        } else {
          write.write(s2.as_bytes()).await.unwrap();
          write.flush().await.unwrap();
          s2.truncate(0);
        }
      }
    }
  });

  let reader = tokio::spawn(async move {
    let read_buf = tokio::io::BufReader::new(read);

    let mut lines = read_buf.lines();

    while let Some(s) = lines.next_line().await.unwrap() {
      tracing::info!(target: "client" , "R {s:?}")
    }
  });

  _ = tokio::join![writer, reader];

  Ok(())
}

async fn client(path: &std::path::Path) {
  tokio::time::sleep(std::time::Duration::from_secs(1)).await;

  if let Ok(conn) = tokio::net::UnixStream::connect(path).await {
    tracing::info!(target :"client" , "client initiated with config: path = {}", path.display());
    match client_write_handler(conn).await {
      Err(err) => {
        if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
          tracing::error!( target: "client", "c: {:?} {}", path, err);
        }
      }

      _ => {
        tracing::warn!(target: "client", "connection satisfied: {path:?}");
      }
    }
  } else {
    tracing::error!(target :"client" , "connection failed");
  }
}

/// Services
mod server {
  /// basic server
  pub struct Server {
    stream: tokio::net::UnixStream,
    read_switch: bool,
  }

  use std::pin::Pin;
  use tower::Service;
  use crate::Frame;

  impl<Request> Service<Request> for Server
  where
    Request: super::IntoRequest + 'static + Send + Sync,
  {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;
    type Response = super::Response;

    fn poll_ready(
      &mut self,
      cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
      if self.read_switch {
        self.stream.poll_read_ready(cx)
      } else {
        self.stream.poll_write_ready(cx)
      }
      .map_err(|e| e.into())
    }

    fn call(&mut self, req: Request) -> Self::Future {
      Box::pin(async move {

        match req.into_frame() {
          Frame::Error(_) =>{}
          Frame::Res(_) => {}
          Frame::Done => {

          }
        };

        Ok(Frame::done())
      })
    }
  }
}

/// REQUEST - RESPONSE
///
/// REQUEST {
///   ERROR, RES
/// }
pub struct Request {
  pub frame: Frame,
}

pub struct Response {
  pub frame: Frame,
}

enum Frame {
  Error(String),
  Res(String),
  Done,
}

trait IntoRequest {
  fn into_frame(self) -> Frame;

  fn into_req(self) -> Request;
}

impl IntoRequest for Request {
  fn into_frame(self) -> Frame {
    self.frame
  }

  fn into_req(self) -> Request {
    self
  }
}

unsafe impl Send for Request {}
unsafe impl Sync for Request {}
unsafe impl Send for Response {}
unsafe impl Sync for Response {}

#[allow(dead_code)]
impl Frame {
  fn new_req(s: String) -> Request {
    Request {
      frame: Frame::Res(s),
    }
  }

  fn new_err(s: String) -> Response {
    Response {
      frame: Frame::Error(s),
    }
  }

  fn new_res(s: String) -> Response {
    Response {
      frame: Frame::Res(s),
    }
  }

  fn done() -> Response {
    Response { frame: Frame::Done }
  }

  fn into_res(self) -> Response {
    Response { frame: self }
  }

  fn into_req(self) -> Request {
    Request { frame: self }
  }
}

#[allow(dead_code)]
impl Request {
  fn io_slice<'a>(&'a self) -> [std::io::IoSlice<'a>; 2] {
    use std::io::IoSlice;
    match &self.frame {
      Frame::Error(e) => [IoSlice::new(&[0u8]), IoSlice::new(e.as_bytes())],
      Frame::Res(r) => [IoSlice::new(&[0u8]), IoSlice::new(r.as_bytes())],
      Frame::Done => [IoSlice::new(&[0u8]), IoSlice::new(&[0u8])],
    }
  }

  fn as_ref(&self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let mut v = vec![1u8];
        v.extend_from_slice(e.as_bytes());
        v
      }
      Frame::Res(r) => {
        let mut v = vec![2u8];
        v.extend_from_slice(r.as_bytes());
        v
      }
      Frame::Done => {
        vec![0u8]
      }
    }
  }
}

#[allow(dead_code)]
impl Response {
  fn io_slice<'a>(&'a self) -> [std::io::IoSlice<'a>; 2] {
    use std::io::IoSlice;
    match &self.frame {
      Frame::Error(e) => [IoSlice::new(&[0u8]), IoSlice::new(e.as_bytes())],
      Frame::Res(r) => [IoSlice::new(&[0u8]), IoSlice::new(r.as_bytes())],
      Frame::Done => [IoSlice::new(&[0u8]), IoSlice::new(&[0u8])],
    }
  }

  fn as_ref(&self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let mut v = vec![1u8];
        v.extend_from_slice(e.as_bytes());
        v
      }
      Frame::Res(r) => {
        let mut v = vec![2u8];
        v.extend_from_slice(r.as_bytes());
        v
      }
      Frame::Done => {
        vec![0u8]
      }
    }
  }
}
