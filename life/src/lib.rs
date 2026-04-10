/// Services
pub mod client {
  use crate::Frame;
  use crate::Response;
  use tokio::io::AsyncBufReadExt;
  use tokio::io::AsyncRead;
  use tokio::io::AsyncWrite;
  use tokio::io::AsyncWriteExt;

  #[allow(unused)]
  async fn client_m_handler(
    mut conn: tokio::net::UnixStream,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(target :"client" , "connection established {conn:?}");
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

    conn
      .write(&Frame::new_req(String::from("client hello")).into_vec_u8())
      .await?;
    conn.flush().await?;

    let (mut read, mut write) = conn.into_split();

    let writer: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
      tokio::spawn(async move {
        let mut s2 = String::new();

        loop {
          stdin.read_line(&mut s2).await?;
          s2 = s2.trim_end().to_owned();

          if !s2.is_empty() {
            tracing::debug!(": {:?}", s2);
            if s2 == "exit" {
              write.write(&Frame::done().into_vec_u8()).await?;
              write.flush().await?;
              break;
            } else {
              write
                .write(&Frame::new_req(s2.clone()).into_vec_u8())
                .await?;
              write.flush().await?;
              s2.truncate(0);
            }
          }
        }

        Ok(())
      });

    let reader: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
      tokio::spawn(async move {
        loop {
          let res = Response::read_from(&mut read).await?;
          tracing::info!(target: "ingress" , "{res}");
          match res.frame {
            Frame::Error(_) | Frame::Reset => break,
            _ => {}
          }
        }

        Ok(())
      });

    Ok(tokio::select! {
     _ = reader => {},
     _ = writer => {},
    })
  }

  pub async fn handshake<T: AsyncWrite + AsyncRead + Unpin>(
    mut conn: T,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    conn
      .write_all(&Frame::new_req(String::from("client hello")).into_vec_u8())
      .await?;
    conn.flush().await?;

    let res: Result<Response, Box<dyn std::error::Error + Send + Sync>> = tokio::select! {
      res = Response::read_from(&mut conn) => { res },
      _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
        Err(Box::new(
                std::io::Error::new(std::io::ErrorKind::TimedOut, "handshake timeout")
              ).into())
      }
    };

    match res?.frame {
      Frame::Error(e) => Err(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::TimedOut,
          format!("handshake disruped due to {}", e),
        ))
        .into(),
      ),

      Frame::Reset => Err(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::TimedOut,
          "handshake failed due to reset",
        ))
        .into(),
      ),

      Frame::Res(s) => {
        if s == "server hello" {
          tracing::info!(target: "client","*** handshake completed");
          Ok(())
        } else {
          Err(
            Box::new(std::io::Error::new(
              std::io::ErrorKind::TimedOut,
              format!("handshake disruped: {}", s),
            ))
            .into(),
          )
        }
      }
    }
  }

  pub async fn client_handler<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    mut conn: T,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(target :"client" , "*** connection established");
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut s2 = String::new();

    handshake(&mut conn).await?;

    loop {
      stdout.write(b"SEND > ").await?;
      stdout.flush().await?;

      stdin.read_line(&mut s2).await?;
      s2 = s2.trim_end().to_owned();

      if !s2.is_empty() {
        if s2 == "exit" {
          conn.write(&Frame::done().into_vec_u8()).await?;
          conn.flush().await?;
          break;
        } else {
          conn
            .write(&Frame::new_req(s2.clone()).into_vec_u8())
            .await?;
          conn.flush().await?;
        }
        tracing::debug!(target: "client","*** completely send off, {:?}", s2);
        s2.truncate(0);
      } else {
        continue;
      }

      let res = Response::read_from(&mut conn).await?;
      tracing::info!(target: "ingress" , "{res}");
      match res.frame {
        Frame::Error(_) | Frame::Reset => break,
        _ => {}
      }
    }

    Ok(())
  }
}
pub mod server {
  /// basic server
  pub struct Server<T> {
    stream: T,
  }

  use crate::Frame;
  use crate::Request;
  use std::fmt::Display;
  use std::pin::Pin;
  use tokio::io::AsyncRead;
  use tokio::io::AsyncWrite;
  use tokio::io::AsyncWriteExt;
  use tower::Service;

  impl<T> Server<T>
  where
    T: AsyncRead + AsyncWrite + Unpin,
  {
    pub fn new(stream: T) -> Self {
      Self { stream }
    }

    pub async fn handle(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
      loop {
        let req = Request::read_from(&mut self.stream).await?;
        let res = self.call(req).await;
        match res {
          Err(e) => {
            self
              .stream
              .write_all(&Frame::new_err(e.to_string()).into_vec_u8())
              .await?;
          }
          Ok(res) => {
            tracing::info!(target: "engress", "<<< {}", res);
            self.stream.write_all(&res.into_vec_u8()).await?;
            match res.frame {
              Frame::Reset | Frame::Error(_) => break,
              _ => {}
            }
          }
        };
        self.stream.flush().await?;
        tracing::debug!(target : "server","*** response completely send off");
      }

      Ok(())
    }
  }

  impl<Request, T> Service<Request> for Server<T>
  where
    Request: super::IntoRequest + 'static + Send + Sync + Display,
  {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;
    type Response = super::Response;

    fn poll_ready(
      &mut self,
      _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
      // self.stream.poll_read_ready(cx).map_err(|e| e.into())
      //
      std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
      tracing::info!(target: "ingress", ">>> {}", req);

      Box::pin(async move {
        let res = match req.into_frame() {
          Frame::Error(_) => Frame::new_err("client error".to_owned()),
          Frame::Res(s) => match s.as_str() {
            "client hello" => Frame::new_res("server hello".to_owned()),

            cmd if cmd.starts_with("say ") => {
              tracing::info!(target: "service" , "echoing: {}", cmd.replace("says ", ""));
              Frame::new_res("ok".to_owned())
            }

            _ => {
              tracing::debug!( target: "server" ,"what: {:?}", s);
              Frame::new_err("what ?".to_owned())
            }
          },

          Frame::Reset => Frame::done(),
        };

        Ok(res)
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

pub enum Frame {
  Error(String),
  Res(String),
  Reset,
}

pub trait IntoRequest {
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

impl Frame {
  pub fn new_req(s: String) -> Request {
    Request {
      frame: Frame::Res(s),
    }
  }

  pub fn new_err(s: String) -> Response {
    Response {
      frame: Frame::Error(s),
    }
  }

  pub fn new_res(s: String) -> Response {
    Response {
      frame: Frame::Res(s),
    }
  }

  pub fn done() -> Response {
    Response {
      frame: Frame::Reset,
    }
  }

  pub fn into_res(self) -> Response {
    Response { frame: self }
  }

  pub fn into_req(self) -> Request {
    Request { frame: self }
  }
}

#[allow(dead_code)]
impl Request {
  pub fn into_vec_u8<'a>(&'a self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let bl = e.as_bytes().len();

        let length = [
          ((bl >> 56) as u8),
          ((bl >> 48) as u8),
          ((bl >> 40) as u8),
          ((bl >> 32) as u8),
          ((bl >> 24) as u8),
          ((bl >> 16) as u8),
          ((bl >> 8) as u8),
          (bl as u8),
        ];

        let mut data = Vec::with_capacity(bl + 9);

        data.extend_from_slice(&length);
        data.push(1);
        data.extend_from_slice(&e.as_bytes());

        data
      }
      Frame::Res(r) => {
        let bl = r.as_bytes().len();

        let mut data = Vec::with_capacity(bl + 9);

        let length = [
          ((bl >> 56) as u8),
          ((bl >> 48) as u8),
          ((bl >> 40) as u8),
          ((bl >> 32) as u8),
          ((bl >> 24) as u8),
          ((bl >> 16) as u8),
          ((bl >> 8) as u8),
          (bl as u8),
        ];

        data.extend_from_slice(&length);
        data.push(2);
        data.extend_from_slice(&r.as_bytes());
        data
      }

      Frame::Reset => vec![0; 9],
    }
  }

  fn from<'a>(d: &'a [u8]) -> Option<Self> {
    let ll = d.get(0..=7)?;

    let length: usize = ((ll[0] as u64)
      | ((ll[1] as u64) << 8)
      | ((ll[2] as u64) << 16)
      | ((ll[3] as u64) << 24)
      | ((ll[4] as u64) << 32)
      | ((ll[5] as u64) << 40)
      | ((ll[6] as u64) << 48)
      | ((ll[7] as u64) << 56)) as usize;

    let t: Frame = match d.get(8)? {
      0 => Frame::Reset,
      1 => {
        let msg: Option<String> = d
          .get(9..length)
          .map(|i| str::from_utf8(i).ok())
          .flatten()
          .map(String::from);
        Frame::Error(msg?)
      }
      2 => {
        let msg: Option<String> = d
          .get(9..length)
          .map(|i| str::from_utf8(i).ok())
          .flatten()
          .map(String::from);
        Frame::Res(msg?)
      }
      _ => Frame::Error(format!("unknown type")),
    };

    Some(Self { frame: t })
  }

  pub async fn read_from<T>(mut stream: T) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
  where
    T: tokio::io::AsyncReadExt + Unpin,
  {
    let mut buf = [0u8; 9];

    stream.read_exact(&mut buf).await?;

    let length: usize = ((buf[7] as u64)
      | ((buf[6] as u64) << 8)
      | ((buf[5] as u64) << 16)
      | ((buf[4] as u64) << 24)
      | ((buf[3] as u64) << 32)
      | ((buf[2] as u64) << 40)
      | ((buf[1] as u64) << 48)
      | ((buf[0] as u64) << 56)) as usize;

    if length > 2usize.pow(20) {
      return Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::OutOfMemory,
        format!(
          "cannot allocate 0x{:0>16x?} or {} Mb",
          length,
          length / 1024 * 1024
        ),
      )));
    }

    match buf[8] {
      0 => Ok(Frame::Reset),
      1 => {
        let mut buf2 = vec![0u8; length];
        stream.read_exact(&mut buf2).await?;
        Ok(Frame::Error(String::from_utf8(buf2)?))
      }
      2 => {
        let mut buf2 = vec![0u8; length];
        stream.read_exact(&mut buf2).await?;
        Ok(Frame::Res(String::from_utf8(buf2)?))
      }
      _ => Err(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::ConnectionReset,
          "cannot parse this message",
        ))
        .into(),
      ),
    }
    .map(|i| Self { frame: i })
  }
}

#[allow(dead_code)]
impl Response {
  pub fn is_err(&self) -> bool {
    match &self.frame {
      Frame::Error(_) => true,
      _ => false,
    }
  }

  pub fn into_vec_u8<'a>(&'a self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let bl = e.as_bytes().len();

        let length = [
          ((bl >> 56) as u8),
          ((bl >> 48) as u8),
          ((bl >> 40) as u8),
          ((bl >> 32) as u8),
          ((bl >> 24) as u8),
          ((bl >> 16) as u8),
          ((bl >> 8) as u8),
          (bl as u8),
        ];

        let mut data = Vec::with_capacity(bl + 9);

        data.extend_from_slice(&length);
        data.push(1);
        data.extend_from_slice(&e.as_bytes());

        data
      }
      Frame::Res(r) => {
        let bl = r.as_bytes().len();

        let mut data = Vec::with_capacity(bl + 9);

        let length = [
          ((bl >> 56) as u8),
          ((bl >> 48) as u8),
          ((bl >> 40) as u8),
          ((bl >> 32) as u8),
          ((bl >> 24) as u8),
          ((bl >> 16) as u8),
          ((bl >> 8) as u8),
          (bl as u8),
        ];

        data.extend_from_slice(&length);
        data.push(2);
        data.extend_from_slice(&r.as_bytes());
        data
      }

      Frame::Reset => vec![0; 9],
    }
  }

  fn from<'a>(d: &'a [u8]) -> Option<Self> {
    let t: Frame = match d.get(0)? {
      0 => Frame::Reset,
      1 => {
        let msg: Option<String> = d
          .get(1..)
          .map(|i| str::from_utf8(i).ok())
          .flatten()
          .map(String::from);
        Frame::Error(msg?)
      }
      2 => {
        let msg: Option<String> = d
          .get(1..)
          .map(|i| str::from_utf8(i).ok())
          .flatten()
          .map(String::from);
        Frame::Res(msg?)
      }
      _ => Frame::Error(format!("unknown type")),
    };

    Some(Self { frame: t })
  }

  pub async fn read_from<T>(mut stream: T) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
  where
    T: tokio::io::AsyncReadExt + Unpin,
  {
    let mut buf = [0u8; 9];

    stream.read_exact(&mut buf).await?;

    let length: usize = ((buf[7] as u64)
      | ((buf[6] as u64) << 8)
      | ((buf[5] as u64) << 16)
      | ((buf[4] as u64) << 24)
      | ((buf[3] as u64) << 32)
      | ((buf[2] as u64) << 40)
      | ((buf[1] as u64) << 48)
      | ((buf[0] as u64) << 56)) as usize;

    if length > 2usize.pow(20) {
      return Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::OutOfMemory,
        format!(
          "cannot allocate 0x{:0>16x?} or {} Mb",
          length,
          length / 1024 * 1024
        ),
      )));
    }

    match buf[8] {
      0 => Ok(Frame::Reset),
      1 => {
        let mut buf2 = vec![0u8; length];
        stream.read_exact(&mut buf2).await?;
        Ok(Frame::Error(String::from_utf8(buf2)?))
      }
      2 => {
        let mut buf2 = vec![0u8; length];
        stream.read_exact(&mut buf2).await?;
        Ok(Frame::Res(String::from_utf8(buf2)?))
      }
      _ => Err(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::ConnectionReset,
          "cannot parse this message",
        ))
        .into(),
      ),
    }
    .map(|i| Self { frame: i })
  }
}

impl std::fmt::Display for Response {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match &self.frame {
        Frame::Reset => format!("RESET"),
        Frame::Res(s) => format!("RES: {s}"),
        Frame::Error(s) => format!("Error: {s}"),
      }
    )
  }
}

impl std::fmt::Display for Request {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match &self.frame {
        Frame::Reset => format!("RESET"),
        Frame::Res(s) => format!("RES: {s}"),
        Frame::Error(s) => format!("Error: {s}"),
      }
    )
  }
}
