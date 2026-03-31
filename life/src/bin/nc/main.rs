#[tokio::main(flavor = "current_thread")]
async fn main() {
  server("/tmp/sock.0".into()).await;
}

async fn server(path: std::path::PathBuf) {
  let listener = tokio::net::UnixListener::bind(path).unwrap();

  loop {
    let stream = listener.accept().await;
    tokio::spawn(async move {
      if let Ok((stream, addr)) = stream {
        tracing::info!("connect established: {:?}", addr);
        match server_handler(stream).await {
          Err(err) => {
            if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
              tracing::error!("c: {:?} {}", addr, err);
            }
          }

          _=>{}
        }
      } else {
        tracing::error!("connect failed to establish: hard reset")
      }
    });
  }
}

async fn server_handler(_stream: tokio::net::UnixStream) -> Result<() , Box<dyn std::error::Error + Send + Sync>> {
  Ok(())
}
