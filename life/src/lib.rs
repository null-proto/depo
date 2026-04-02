use tokio::io::AsyncReadExt as _;


/// Services
mod server {
  /// basic server
  pub struct Server {
    stream: tokio::net::UnixStream,
  }

  use crate::Frame;
  use std::pin::Pin;
  use tower::Service;

  impl Server {
    fn new(stream: tokio::net::UnixStream) -> Self {
      Self { stream }
    }

    fn handle(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
      todo!()
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
          Frame::Error(_) => Frame::new_err("Client Error".to_owned()),
          Frame::Res(_) => Frame::new_res("ok".to_owned()),
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
    Response {
      frame: Frame::Reset,
    }
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
  fn into_vec_u8<'a>(&'a self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let bl = e.as_bytes().len() + 1;

        let length = [
          (bl as u8),
          ((bl as u8) << 8),
          ((bl as u8) << 16),
          ((bl as u8) << 24),
          ((bl as u8) << 40),
          ((bl as u8) << 48),
          ((bl as u8) << 56),
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
          ((bl as u8) << 8),
          ((bl as u8) << 16),
          ((bl as u8) << 24),
          ((bl as u8) << 32),
          ((bl as u8) << 40),
          ((bl as u8) << 48),
          ((bl as u8) << 56),
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

  pub async fn read_from(
    stream: &mut tokio::net::UnixStream,
  ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
  fn into_vec_u8<'a>(&'a self) -> Vec<u8> {
    match &self.frame {
      Frame::Error(e) => {
        let bl = e.as_bytes().len() + 1;

        let length = [
          (bl as u8),
          ((bl as u8) << 8),
          ((bl as u8) << 16),
          ((bl as u8) << 24),
          ((bl as u8) << 40),
          ((bl as u8) << 48),
          ((bl as u8) << 56),
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
          ((bl as u8) << 8),
          ((bl as u8) << 16),
          ((bl as u8) << 24),
          ((bl as u8) << 40),
          ((bl as u8) << 48),
          ((bl as u8) << 56),
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

  pub async fn read_from(
    stream: &mut tokio::net::UnixStream,
  ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
