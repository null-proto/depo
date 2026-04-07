use std::{fmt::{Display, write}, net::{Ipv4Addr, SocketAddr}};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

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
        i = async {
          stdout.write_all(b" -> {buf}").await;
          stdout.flush().await;
          stdin.read_line(&mut buf).await;
        } => { Event::Io(i) }
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
              Some('x') => Message::Del(buf_s[1..].to_string()),
              Some('a') => Message::Add(buf_s[1..].to_string()),
              Some(_) => Message::Add(buf_s[1..].to_string()),
              None => Message::None,
            };
            sender.send(msg).await;
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
    let _path = server_path;
    let _addr = SocketAddr::new(std::net::IpAddr::V4( Ipv4Addr::new(127, 0,0,1)), 8080);
    let _sender = txr;
    let _receiver = rx;
  });

  tokio::join!(reader, server);
}

enum Event<'a> {
  Io(()),
  Msg(Option<Message<'a>>),
}

#[derive(Debug)]
enum Message<'a> {
  None,
  Ok,
  Logs(&'a str),
  Log(String),
  Add(String),
  Del(String),
}

impl Display for Message<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Message::*;

    write!(
      f,
      "{}",
      match self {
        None => format!("na."),
        Ok => format!("ok"),
        Log(s) => {
          format!("log {}", s)
        }
        Logs(s) => {
          format!("log {}", s)
        }

        Add(s) => {
          format!("add {}", s)
        }
        Del(s) => {
          format!("delete {}", s)
        }
      }
    )
  }
}

unsafe impl Send for Message<'_> {}
