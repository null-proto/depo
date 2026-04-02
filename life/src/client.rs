use life::Frame;
use life::Request;
use life::Response;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");

  tokio::time::sleep(std::time::Duration::from_secs(1)).await;

  if let Ok(conn) = tokio::net::UnixStream::connect(&path).await {
    tracing::info!(target :"client" , "client initiated with config: path = {}", path.display());
    match client_write_handler(conn).await {
      Err(err) => {
        if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
          tracing::error!( target: "client", "c: {:?} {}", path, err);
        }
      }

      _ => {
        tracing::warn!(target: "client", "connection satisfied: {path:?}");
      }
    }
  } else {
    tracing::error!(target :"client" , "connection failed");
  }
}

#[allow(unreachable_code)]
async fn client_write_handler(
  mut conn: tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  tracing::info!(target :"client" , "connection established {conn:?}");
  let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());

  conn.write(b"client hello\n").await?;
  conn.flush().await?;

  let (read, mut write) = conn.into_split();

  let writer = tokio::spawn(async move {
    let mut s2 = String::new();
    loop {
      stdin.read_line(&mut s2).await.unwrap();

      if !s2.is_empty() {
        if s2 == "exit" {
          std::process::exit(0);
        } else {
          write.write(s2.as_bytes()).await.unwrap();
          write.flush().await.unwrap();
          s2.truncate(0);
        }
      }
    }
  });

  let reader = tokio::spawn(async move {
    let read_buf = tokio::io::BufReader::new(read);

    let mut lines = read_buf.lines();

    while let Some(s) = lines.next_line().await.unwrap() {
      tracing::info!(target: "client" , "R {s:?}")
    }
  });

  _ = tokio::join![writer, reader];

  Ok(())
}
