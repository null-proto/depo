/// Services
pub mod server {
  /// basic server
  #[derive(Debug)]
  pub struct Server {
    stream: tokio::net::UnixStream,
  }

  use crate::Frame;
  use crate::Request;
  use std::pin::Pin;
  use tokio::io::AsyncWriteExt;
  use tower::Service;

  impl Server {
    pub fn new(stream: tokio::net::UnixStream) -> Self {
      Self { stream }
    }

    pub async fn handle(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
      loop {
        let req = Request::read_from(&mut self.stream).await?;
        let res = self.call(req).await?;
        let s = res.into_vec_u8();
        self.stream.write_all(&s).await?;
        self.stream.flush().await?;

        match res.frame {
          Frame::Reset | Frame::Error(_) => break,
          _ => {}
        };
      }

      Ok(())
    }

  }

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
      self.stream.poll_read_ready(cx).map_err(|e| e.into())
    }

    fn call(&mut self, req: Request) -> Self::Future {
      Box::pin(async move {
        let res = match req.into_frame() {
          Frame::Error(err) => {
            tracing::warn!("FR:ERR c : {}", err);
            Frame::new_err("Client Error".to_owned())
          }
          Frame::Res(s) => {
            match s.as_str() {
              "client hello" => Frame::new_res(String::from("server hello")),
              s if s.starts_with("echo ") => {
                tracing::info!( target: "server" ,"exec: {}", s);
                Frame::new_res(String::from("ok"))
              },
              _ => Frame::new_err(String::from("not allowed."))
            }
          },

          Frame::Reset => {
            tracing::warn!("FR:RST connection ");
            Frame::done()
          }
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
        let bl = e.as_bytes().len() + 1;

        let length = [
          (bl as u8),
          ((bl >> 8) as u8),
          ((bl >> 16) as u8),
          ((bl >> 24) as u8),
          ((bl >> 40) as u8),
          ((bl >> 48) as u8),
          ((bl >> 56) as u8),
        ];

        let mut data = Vec::with_capacity(bl + 9);

        data.extend_from_slice(&length);
        data.push(1);
        data.extend_from_slice(&e.as_bytes());

        data
      }
      Frame::Res(r) => {
        let bl = r.as_bytes().len() + 1;

        let mut data = Vec::with_capacity(bl + 9);

        let length = [
          (bl as u8),
          ((bl >> 8) as u8),
          ((bl >> 16) as u8),
          ((bl >> 24) as u8),
          ((bl >> 40) as u8),
          ((bl >> 48) as u8),
          ((bl >> 56) as u8),
        ];

        data.extend_from_slice(&length);
        data.push(2);
        data.extend_from_slice(&r.as_bytes());
        data
      }

      Frame::Reset => vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    }
  }

  fn from<'a>(d: &'a [u8]) -> Option<Self> {
    let ll = d.get(0..7)?;

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
    let mut buf = [0u8; 8];

    stream.read_exact(&mut buf).await?;

    let length: usize = ((buf[0] as u64)
      | ((buf[1] as u64) << 8)
      | ((buf[2] as u64) << 16)
      | ((buf[3] as u64) << 24)
      | ((buf[4] as u64) << 32)
      | ((buf[5] as u64) << 40)
      | ((buf[6] as u64) << 48)
      | ((buf[7] as u64) << 56)) as usize;

    let mut buf2 = vec![0u8; length];

    stream.read_exact(&mut buf2).await?;

    Self::from(&buf).ok_or(Box::new(std::io::Error::new(
      std::io::ErrorKind::ConnectionReset,
      "cannot parse this message",
    )))
  }
}

#[allow(dead_code)]
impl Response {
  pub fn into_vec_u8<'a>(&'a self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let bl = e.as_bytes().len() + 1;

        let length = [
          (bl as u8),
          ((bl >> 8) as u8),
          ((bl >> 16) as u8),
          ((bl >> 24) as u8),
          ((bl >> 40) as u8),
          ((bl >> 48) as u8),
          ((bl >> 56) as u8),
        ];

        let mut data = Vec::with_capacity(bl + 9);

        data.extend_from_slice(&length);
        data.push(1);
        data.extend_from_slice(&e.as_bytes());

        data
      }
      Frame::Res(r) => {
        let bl = r.as_bytes().len() + 1;

        let mut data = Vec::with_capacity(bl + 9);

        let length = [
          (bl as u8),
          ((bl >> 8) as u8),
          ((bl >> 16) as u8),
          ((bl >> 24) as u8),
          ((bl >> 40) as u8),
          ((bl >> 48) as u8),
          ((bl >> 56) as u8),
        ];

        data.extend_from_slice(&length);
        data.push(2);
        data.extend_from_slice(&r.as_bytes());
        data
      }

      Frame::Reset => vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
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
    let mut buf = [0u8; 8];

    stream.read_exact(&mut buf).await?;

    let length: usize = ((buf[0] as u64)
      | ((buf[1] as u64) << 8)
      | ((buf[2] as u64) << 16)
      | ((buf[3] as u64) << 24)
      | ((buf[4] as u64) << 32)
      | ((buf[5] as u64) << 40)
      | ((buf[6] as u64) << 48)
      | ((buf[7] as u64) << 56)) as usize;

    let mut buf2 = vec![0u8; length];

    stream.read_exact(&mut buf2).await?;

    Self::from(&buf).ok_or(Box::new(std::io::Error::new(
      std::io::ErrorKind::ConnectionReset,
      "cannot parse this message",
    )))
  }
}
