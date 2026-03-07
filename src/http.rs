#![allow(unused)]

use bytes::Bytes;
use http::{Method, Request};


fn parse_req(data: Bytes) -> Request<Option<Bytes>> {


  let method: Option<Method> = match &data[0..2] {
    b"GET" => Some(Method::GET),

    _ => None
  };

  Request::builder()
    .method(http::Method::GET)
    .uri("/")
    .body(None)
    .unwrap()
}
