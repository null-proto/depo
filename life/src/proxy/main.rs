use std::{
  fmt::{Display, write},
  net::{Ipv4Addr, SocketAddr},
};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::srv::ProxyAgent;

pub mod srv;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .init();

  let _path: Vec<String> = std::env::args().collect();
  let server_path = std::path::PathBuf::from(_path.get(1).expect("put the unix socket path"));

  let (tx, rx) = tokio::sync::mpsc::channel::<Message>(1);
  let (txr, rxr) = tokio::sync::mpsc::channel::<Message>(1);

  let reader = tokio::spawn(async move {
    let sender = tx;
    let mut receiver = rxr;
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut buf = String::new();

    loop {
      let m = tokio::select! {
        s = receiver.recv() => { Event::Msg(s) }
        _ = async {
          _ = stdout.write_all(b" -> {buf}").await;
          _ = stdout.flush().await;
          _ = stdin.read_line(&mut buf).await;
        } => { Event::Io(()) }
      };

      match m {
        Event::Io(_) => {
          let buf_s = buf.trim_end();

          if buf_s.chars().last().map(|i| i == '-').unwrap_or(false) {
            buf.clear();
            continue;
          }

          if !buf_s.is_empty() {
            let msg = match buf_s.chars().nth(0) {
              Some('x') => Message::Del(buf_s[2..].to_string()),
              Some('a') => Message::Add(buf_s[2..].to_string()),
              Some(_) => Message::Add(buf_s[2..].to_string()),
              None => Message::None,
            };
            sender.send(msg).await.expect("cannot send anything to Proxy");
          };

          buf.clear();
        }

        Event::Msg(Some(m)) => {
          println!("server says: {m}")
        }
        _ => {}
      };
    }
  });

  let server = tokio::spawn(async move {
    let path = server_path;
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let sender = txr;
    let receiver = rx;

    let ca = ProxyAgent::new(path, addr , sender, receiver);
  });

  _ = tokio::join!(reader, server);
}

enum Event {
  Io(()),
  Msg(Option<Message>),
}

#[derive(Debug)]
#[allow(unused)]
pub enum Message {
  None,
  Ok,
  Log(String),
  Add(String),
  Del(String),
}

impl Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Message::*;

    write!(
      f,
      "{}",
      match self {
        None => format!("na."),
        Ok => format!("ok"),
        Log(s) => {
          format!("log \x1b[39m{}\x1b[0m", s)
        }
        Add(s) => {
          format!("add \x1b[33m{}\x1b[0m", s)
        }
        Del(s) => {
          format!("delete \x1b[31m{}\x1b[0m", s)
        }
      }
    )
  }
}

unsafe impl Send for Message {}
