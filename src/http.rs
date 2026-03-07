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
    // cuz of using windowing in previous step, offset should added
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
