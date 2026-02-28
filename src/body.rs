use std::{
  borrow::Cow, convert::Infallible, pin::Pin, task::{Context, Poll}
};

use bytes::{Buf, Bytes};
use hyper::body::{Body as HyprBody, Frame, SizeHint};

#[derive(Clone, Copy, Debug)]
#[pin_project::pin_project]
pub struct HttpBody<D> {
  data: Option<D>
}

impl<D> HttpBody<D>
where
  D: Buf,
{
  /// Create a new [`HttpBody`] with data.
  pub fn new(data: D) -> Self {
    let data = if data.has_remaining() {
      Some(data)
    } else {
      None
    };
    HttpBody { data }
  }

  /// Create a new empty [`HttpBody`].
  pub fn empty() -> Self {
    Default::default()
  }
}

impl<D> HyprBody for HttpBody<D>
where
  D: Buf,
{
  type Data = D;
  type Error = Infallible;

  fn poll_frame(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<Option<Result<Frame<D>, Self::Error>>> {
    Poll::Ready(self.data.take().map(|d| Ok(Frame::data(d))))
  }

  fn is_end_stream(&self) -> bool {
    self.data.is_none()
  }

  fn size_hint(&self) -> SizeHint {
    self
      .data
      .as_ref()
      .map(|data| SizeHint::with_exact(u64::try_from(data.remaining()).unwrap()))
      .unwrap_or_else(|| SizeHint::with_exact(0))
  }
}

impl<D> Default for HttpBody<D>
where
  D: Buf,
{
  /// Create an empty [`HttpBody`].
  fn default() -> Self {
    HttpBody { data: None }
  }
}

impl<D> From<Bytes> for HttpBody<D>
where
  D: Buf + From<Bytes>,
{
  fn from(bytes: Bytes) -> Self {
    HttpBody::new(D::from(bytes))
  }
}

impl<D> From<Vec<u8>> for HttpBody<D>
where
  D: Buf + From<Vec<u8>>,
{
  fn from(vec: Vec<u8>) -> Self {
    HttpBody::new(D::from(vec))
  }
}

impl<D> From<&'static [u8]> for HttpBody<D>
where
  D: Buf + From<&'static [u8]>,
{
  fn from(slice: &'static [u8]) -> Self {
    HttpBody::new(D::from(slice))
  }
}

impl<D, B> From<Cow<'static, B>> for HttpBody<D>
where
  D: Buf + From<&'static B> + From<B::Owned>,
  B: ToOwned + ?Sized,
{
  fn from(cow: Cow<'static, B>) -> Self {
    match cow {
      Cow::Borrowed(b) => HttpBody::new(D::from(b)),
      Cow::Owned(o) => HttpBody::new(D::from(o)),
    }
  }
}

impl<D> From<String> for HttpBody<D>
where
  D: Buf + From<String>,
{
  fn from(s: String) -> Self {
    HttpBody::new(D::from(s))
  }
}

impl<D> From<&'static str> for HttpBody<D>
where
  D: Buf + From<&'static str>,
{
  fn from(slice: &'static str) -> Self {
    HttpBody::new(D::from(slice))
  }
}
