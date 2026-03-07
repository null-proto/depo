#![allow(unused)]

use bytes::Bytes;
use http::HeaderMap;
use http::{Method, Request};

pub fn parse_req(data: Bytes) -> Result<Request<Option<Bytes>>, http::Error> {
  let (status, status_len) = {
    let mut j = 0usize;
    for i in data.windows(2) {
      if i == b"\r\n" {
        break;
      } else {
        j += 1;
      }
    }

    let s = core::str::from_utf8(&data[..j]).unwrap();
    (s.split(" ").collect::<Vec<_>>(), j)
  };

  let (header_map, hm_len) = {
    let mut j = status_len;
    for i in data.windows(4) {
      if i == b"\r\n\r\n" {
        break;
      } else {
        j += 1;
      }
    }

    let s = core::str::from_utf8(&data[status_len + 2..j]).unwrap();

    let s = s
      .split("\r\n")
      .map(|i| i.split_once(':'))
      .flatten()
      .filter(|i| !(i.0.is_empty() || i.1.is_empty()) )
      .map(|(k, v)| (k.trim(), v.trim()))
      .collect::<Vec<(_, _)>>();

    use http::{HeaderName, HeaderValue};

    let mut header_map = HeaderMap::new();
    for (i, j) in s {
      let j = HeaderValue::from_str(j)?;
      header_map.insert(i.parse::<HeaderName>()?, j);
    }

    (header_map, j)
  };

  let body = if let Some(Ok(Ok(content_len))) = header_map
    .get(http::header::CONTENT_LENGTH)
    .map(|i| i.to_str().map(|i| i.parse::<usize>()))
  {
    // cuz of using windowing in previous step, offset was added
    let body_start = hm_len -11;

    Some(data.slice(body_start..body_start + content_len))
  } else {
    None
  };

  let mut req = Request::builder()
    .method(status[0])
    .uri(status[1])
    .body(body)?;

  std::mem::replace(req.headers_mut(), header_map);

  Ok(req)
}

pub trait IntoBytes {
  fn into_h1_bytes(self) -> Vec<u8>;
}

impl<T> IntoBytes for http::Response<Option<T>>
where
  T: AsRef<[u8]>
{
  fn into_h1_bytes(self) -> Vec<u8> {
    use http::version::Version;


    let hm = self.headers();
    let version = self.version();
    let status = self.status();
    let status_text = "Ok";

    let mut _res: Vec<u8> = vec![];

    _res.extend_from_slice( match version {
      Version::HTTP_11 => "HTTP/1.1 ",
      Version::HTTP_10 => "HTTP/1.0 ",

      // not possible
      Version::HTTP_2 => "H2 ",
      Version::HTTP_3 => "H3 ",
      _ => "HTTP/0.9 "
    }.as_bytes() );

    _res.extend_from_slice(
      status.as_str().as_bytes()
    );

    _res.push(' ' as u8);
    _res.extend_from_slice( status.canonical_reason().unwrap_or("Server error").as_bytes() );
    _res.extend_from_slice(b"\r\n");


    let _hm_list = hm.iter()
      .map(|i| {
        [ i.0.as_str().as_bytes() , i.1.as_bytes() ].join(b": " as &[u8])
      })
    .collect::<Vec<Vec<u8>>>();

    _res.extend_from_slice( &_hm_list.join(b"\r\n" as &[u8]) );
    _res.extend_from_slice(b"\r\n\r\n");

    if let Some(body) = self.body() {
      let _a: &[u8] = body.as_ref();

      if !_a.is_empty() {
        _res.extend_from_slice(_a);
        _res.extend_from_slice(b"\r\n");
      }
    }
    tracing::info!("res: {}", String::from_utf8_lossy(&_res));

    _res
  }
}
