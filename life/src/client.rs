use life::Frame;
use life::Response;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");

  if !path.exists() {
    panic!("unix socket not found at {}", path.display());
  }

  let mut reconnect = false;

  loop {
    if reconnect {
      tracing::info!("Retrying in 2s");
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    reconnect = if let Ok(conn) = tokio::net::UnixStream::connect(&path).await {
      if let Err(e) = client_handler(conn).await {
        tracing::error!("*** connection exit, {}", e.to_string());
        true
      } else {
        tracing::warn!("*** connection exit");
        break;
      }
    } else {
      tracing::error!("??? cannot connect");
      true
    }
  }
}

#[allow(unused)]
async fn client_m_handler(
  mut conn: tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing::info!(target :"client" , "connection established {conn:?}");
  let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

  conn
    .write(&Frame::new_req(String::from("client hello")).into_vec_u8())
    .await?;
  conn.flush().await?;

  let (mut read, mut write) = conn.into_split();

  let writer: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
    tokio::spawn(async move {
      let mut s2 = String::new();

      loop {
        stdin.read_line(&mut s2).await?;
        s2 = s2.trim_end().to_owned();

        if !s2.is_empty() {
          tracing::debug!(": {:?}", s2);
          if s2 == "exit" {
            write.write(&Frame::done().into_vec_u8()).await?;
            write.flush().await?;
            break;
          } else {
            write
              .write(&Frame::new_req(s2.clone()).into_vec_u8())
              .await?;
            write.flush().await?;
            s2.truncate(0);
          }
        }
      }

      Ok(())
    });

  let reader: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
    tokio::spawn(async move {
      loop {
        let res = Response::read_from(&mut read).await?;
        tracing::info!(target: "ingress" , "{res}");
        match res.frame {
          Frame::Error(_) | Frame::Reset => break,
          _ => {}
        }
      }

      Ok(())
    });

  Ok(tokio::select! {
   _ = reader => {},
   _ = writer => {},
  })
}

async fn client_handler(
  mut conn: tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing::info!(target :"client" , "*** connection established");
  let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
  let mut stdout = tokio::io::stdout();
  let mut s2 = String::new();

  conn
    .write(&Frame::new_req(String::from("client hello")).into_vec_u8())
    .await?;
  conn.flush().await?;

  let res = Response::read_from(&mut conn).await?;
  tracing::info!(target: "ingress" , "{res}");

  match res.frame {
    Frame::Error(e) => {
      tracing::error!("*** handshake error, {}", e);
    }

    Frame::Reset => {
      tracing::info!("*** handshake disruped, RESET");
    }

    Frame::Res(s) => {
      if s == "server hello" {
        tracing::info!("*** handshake completed");
      } else {
        tracing::info!("*** handshake disruped , {}", s);
      }
    }
  };

  loop {
    stdout.write(b" SEND > ").await?;
    stdout.flush().await?;

    stdin.read_line(&mut s2).await?;
    s2 = s2.trim_end().to_owned();

    if !s2.is_empty() {
      if s2 == "exit" {
        conn.write(&Frame::done().into_vec_u8()).await?;
        conn.flush().await?;
        break;
      } else {
        conn
          .write(&Frame::new_req(s2.clone()).into_vec_u8())
          .await?;
        conn.flush().await?;
      }
    }
    tracing::debug!("*** completely send off, {:?}", s2);
    s2.truncate(0);

    let res = Response::read_from(&mut conn).await?;
    tracing::info!(target: "ingress" , "{res}");
    match res.frame {
      Frame::Error(_) | Frame::Reset => break,
      _ => {}
    }
  }

  Ok(())
}
