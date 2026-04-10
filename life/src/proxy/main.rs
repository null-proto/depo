use std::fmt::Display;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

use crate::srv::Macher;
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
  let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

  println!("\x1b[90mproxy configured\x1b[0m");
  println!("\x1b[32mtcp\x1b[0m   <<<  \x1b[90m{}\x1b[0m", addr);
  println!(
    "\x1b[34munix\x1b[0m  >>>  \x1b[90m{}\x1b[0m",
    server_path.display()
  );

  let (tx, rx) = tokio::sync::mpsc::channel::<Message>(100);
  let (txr, rxr) = tokio::sync::mpsc::channel::<Message>(100);
  let (wtx, wrx) = tokio::sync::watch::channel::<srv::Macher>(Macher::new(""));

  let stdio = tokio::spawn(async move {
    let sender = tx;
    let watcher = wtx;
    let mut receiver = rxr;
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut buf = String::new();
    let mut buf2 = String::new();
    let mut blocked: Vec<Block> = vec![];

    sender.send(Message::Ok).await.unwrap();
    match receiver.recv().await.unwrap() {
      state => stdout
        .write_all(format!("\rserver connection: {state}\n").as_bytes())
        .await
        .unwrap(),
    };

    loop {
      let m = tokio::select! {
        s = receiver.recv() => { Event::Msg(s) }
        _ = async {
          if blocked.is_empty() {
            _ = stdout.write_all(format!("\r\x1b[33m#\x1b[0m {buf}").as_bytes()).await;
            _ = stdout.flush().await;
            _ = stdin.read_line(&mut buf).await;
          } else {
            for (i, b) in blocked.iter().enumerate() {
              _ = stdout.write_all( format!("\r\x1b[95m{:>4}\x1b[0m : \x1b[94m{}\x1b[0m\n" , i , b).as_bytes() ).await;
            }
            _ = stdout.write_all(format!("\r[\x1b[30m{}\x1b[0m]\x1b[31m$\x1b[0m {buf2}", blocked.len()).as_bytes()).await;
            _ = stdout.flush().await;
            _ = stdin.read_line(&mut buf2).await;
          }
        } => { Event::Io(()) }
      };

      match m {
        Event::Io(_) => {
          if blocked.is_empty() {
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

                  _ => watcher.send(Macher::new(buf_s)).unwrap(),
                };
              };
            };
            buf.clear();
          } else {
            let (ind, f) = buf2
              .split_once(" ")
              .map(|(i, j)| (i.parse::<usize>().ok().unwrap_or(0), j.trim()))
              .unwrap_or((0usize, buf2.trim()));

            if match f {
              "c" | "con" | "continue" => {
                let s = blocked.remove(ind).sender;
                s.send(Message::Release)
              }
              "k" | "kill" => {
                let s = blocked.remove(ind).sender;
                s.send(Message::Kill)
              }
              _ => Ok(()),
            }
            .is_err()
            {
              _ = stdout.write_all(b"failed to send\n").await;
            }

            buf2.clear();
          }
        }

        Event::Msg(Some(m)) => {
          println!("\r\x1b[90mserver says:\x1b[0m {m}");
          if let Message::Blocked(block) = m {
            blocked.push(block);
          }
        }
        _ => {}
      };
    }
  });

  let server = tokio::spawn(async move {
    let path = server_path;
    let sender = txr;
    let receiver = rx;

    let mut ca = ProxyAgent::new(path, addr, sender, receiver, wrx);
    ca.runner().await.unwrap();
  });

  _ = tokio::join!(stdio, server);
}

enum Event {
  Io(()),
  Msg(Option<Message>),
}

#[allow(unused)]
pub enum Message {
  None,
  Ok,
  Err,

  Release,
  Kill,

  Blocked(Block),

  Log(String),
  ReloadWatcher,
}

pub struct Block {
  sender: tokio::sync::oneshot::Sender<Message>,
  matcher: String,
}

impl Display for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.matcher)
  }
}

impl Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Message::*;

    write!(
      f,
      "{}",
      match self {
        None => format!("\x1b[33mna.\x1b[0m"),
        Ok => format!("\x1b[32mok\x1b[0m"),
        Err => format!("\x1b[31mer.\x1b[0m"),
        ReloadWatcher => format!("reload-watcher"),
        Release => format!("release"),
        Kill => format!("\x1b[31mkill\x1b[0m"),
        Log(s) => {
          format!("\x1b[30mlog, {}\x1b[0m", s)
        }
        Blocked(Block { matcher, .. }) => {
          format!("block on, \x1b[31m{}\x1b[0m", matcher)
        }
      }
    )
  }
}

unsafe impl Send for Message {}
