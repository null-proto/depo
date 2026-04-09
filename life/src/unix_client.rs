use life::client::client_handler;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let _path: Vec<String> = std::env::args().collect();
  let path = std::path::PathBuf::from(_path.get(1).expect("put the unix socket path"));

  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .init();

  if !path.exists() {
    panic!("unix socket not found at {}", path.display());
  } else {
    tracing::info!(target: "client","Unix Client configured : {}", path.display());
  }

  let mut reconnect = false;

  loop {
    if reconnect {
      tracing::info!(target: "client","Retrying in 2s");
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    reconnect = if let Ok(conn) = tokio::net::UnixStream::connect(&path).await {
      if let Err(e) = client_handler(conn).await {
        tracing::error!(target: "client","*** connection exit, {}", e.to_string());
        true
      } else {
        tracing::warn!(target: "client","*** connection exit");
        break;
      }
    } else {
      tracing::error!(target: "client","??? cannot connect");
      true
    }
  }
}
