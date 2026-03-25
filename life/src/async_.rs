use futures::SinkExt;
use futures::channel;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::mpsc::UnboundedSender;
use std::time::Duration;

use crate::prelud::*;
use crate::ret::Return;

async fn receiver(mut rx: UnboundedReceiver<Message>) -> Signel {
  loop {
    let inp = rx.recv().await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    match inp.as_ref() {
      "exit" => {
        tracing::info!("receiver cannot read anymore: exit call ***");
        break;
      }

      data => tracing::info!("receiver ingress: {}", data),
    }
  }

  Err(Box::new(std::io::Error::new(
    std::io::ErrorKind::TimedOut,
    "Task 1 Finished",
  )))
}

async fn transmitter(mut tx: UnboundedSender<Message>) -> Signel {
  tokio::time::sleep(Duration::from_secs(2));

  loop {
    let inp = readln().await?;

    match inp.as_ref() {
      "exit" => {
        tx.send(Message::Frame(inp)).await?;
        tracing::info!("completely sendoff ***");
        break;
      }

      "" => continue,

      _ => {}
    };
    tx.send(Message::Frame(inp)).await?;
    tracing::info!("completely sendoff ***");
  }

  tracing::info!("transmitter stopped ***");
  Return::info("transmitter ended ***")
}

enum Message {
  Frame(String),
  Error(String),
}

impl AsRef<str> for Message {
  fn as_ref(&self) -> &str {
    match self {
      Message::Frame(s) => &s,
      Message::Error(s) => &s,
    }
  }
}

pub async fn main() -> Signel {
  let (tx, rx) = channel::mpsc::unbounded::<Message>();

  let task1 = tokio::spawn(async move { receiver(rx).await });

  let task2 = tokio::spawn(async move { transmitter(tx).await });

  tokio::join!(task1, task2).1?

  // Return::ok()

  // if let Ok(e) = task1.await {
  //   e.await
  // } else {
  //   Return::fail()
  // }
}
