use life::Frame;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
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
      client_handler(conn).await.is_err()
    } else {
      tracing::error!("connection failed");
      true
    }
  }
}

async fn client_handler(
  mut conn: tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing::info!(target :"client" , "connection established {conn:?}");
  let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

  conn.write( &Frame::new_req(String::from("client hello")).into_vec_u8()).await?;
  conn.flush().await?;

  let (read, mut write) = conn.into_split();

  let writer: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
    tokio::spawn(async move {
      let mut s2 = String::new();

      loop {
        stdin.read_line(&mut s2).await?;

        if !s2.is_empty() {
          if s2 == "exit" {
            write.write(&Frame::done().into_vec_u8()).await?;
            write.flush().await?;
            break;
          } else {
            write.write( &Frame::new_req(s2.clone()).into_vec_u8()).await?;
            write.flush().await?;
            s2.truncate(0);
          }
        }
      }

      Ok(())
    });

  let reader: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
    tokio::spawn(async move {
      let read_buf = tokio::io::BufReader::new(read);
      let mut lines = read_buf.lines();
      while let Some(s) = lines.next_line().await? {
        tracing::info!(target: "client" , "R {s:?}")
      }

      Ok(())
    });

  Ok(tokio::select! {
   _ = reader => {},
   _ = writer => {},
  })
}
