use std::fmt::Display;
use std::net::SocketAddr;
use std::net::Ipv4Addr;

use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;

use crate::srv::ProxyAgent;
use crate::srv::Macher;

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
  let (wtx, wrx) = tokio::sync::watch::channel::<srv::Macher>(Macher::new(""));

  let reader = tokio::spawn(async move {
    let sender = tx;
    let watcher = wtx;
    let mut receiver = rxr;
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut buf = String::new();
    let mut buf2 = String::new();
    let mut blocked: Vec<Block> = vec![];

    loop {
      let m = tokio::select! {
        s = receiver.recv() => { Event::Msg(s) }
        _ = async {
          if blocked.is_empty() {
            _ = stdout.write_all(b" -> {buf}").await;
            _ = stdout.flush().await;
            _ = stdin.read_line(&mut buf).await;
          } else {
            for (i, b) in blocked.iter().enumerate() {
              _ = stdout.write_all( format!("{}: {}\n" , i , b).as_bytes() );
            }
            _ = stdout.write_all(b" -> ").await;
            _ = stdout.flush().await;
            _ = stdin.read_line(&mut buf2).await;

            if let Some((ind , f )) = buf2.split_once(" ").map(|(i,j)| (i.parse::<usize>().ok(),j) ) {
              if let Some(ind) = ind {
                if match f {
                  "c" | "con" | "continue" => {
                    let s = blocked.remove(ind).sender;
                    s.send(Message::Release)
                  },
                  "k" | "kill" => {
                    let s = blocked.remove(ind).sender;
                    s.send(Message::Kill)
                  }
                  _ => {
                    _ = stdout.write_all(b"unknown command: {k}\n").await;
                    _ = stdout.flush().await;
                    Ok(())
                  }
                }.is_err() {
                  _ = stdout.write_all(b"failed to send\n").await;
                }
              }
            }
          }
        } => { Event::Io(()) }
      };

      if blocked.is_empty() {
        match m {
          Event::Io(_) => {
            let buf_s = buf.trim();

            if buf_s.chars().last().map(|i| i == '-').unwrap_or(false) {
              buf.clear();
              continue;
            }

            if !buf_s.is_empty() {
              if let Some(mark) = buf_s.chars().nth(0) {
                match mark {
                  'x' => watcher.send(Macher::new("")).unwrap(),

                  'a' => watcher.send(Macher::new(&buf_s[2..])).unwrap(),

                  's' => sender.send(Message::Ok).await.unwrap(),

                  any => watcher.send(Macher::new(any)).unwrap(),
                };
              };
            };
            buf.clear();
          }

          Event::Msg(Some(m)) => {
            println!("server says: {m}")
          }
          _ => {}
        };
      };
    }
  });

  let server = tokio::spawn(async move {
    let path = server_path;
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let sender = txr;
    let receiver = rx;

    let mut ca = ProxyAgent::new(path, addr, sender, receiver, wrx);
    ca.runner().await.unwrap();
  });

  _ = tokio::join!(reader, server);
}

enum Event {
  Io(()),
  Msg(Option<Message>),
}

#[allow(unused)]
pub enum Message {
  None,
  Ok,

  Release,
  Kill,

  Blocked(Block),

  Log(String),
  ReloadWatcher,
  Add(String),
  Del(String),
}

pub struct Block {
  sender: tokio::sync::oneshot::Sender<Message>,
  matcher: Macher,
}

impl Display for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.matcher.inner)
  }
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
        ReloadWatcher => format!("reload-watcher"),
        Release => format!("release"),
        Kill => format!("kill"),
        Log(s) => {
          format!("log \x1b[39m{}\x1b[0m", s)
        }
        Add(s) => {
          format!("add \x1b[33m{}\x1b[0m", s)
        }
        Del(s) => {
          format!("delete \x1b[31m{}\x1b[0m", s)
        }
        Blocked(Block {
          matcher: Macher { inner },
          ..
        }) => {
          format!("block \x1b[31m{}\x1b[0m", inner)
        }
      }
    )
  }
}

unsafe impl Send for Message {}
