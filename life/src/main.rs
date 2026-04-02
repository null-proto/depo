// #[tokio::main(flavor = "multi_thread")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");

  let listener = tokio::net::UnixListener::bind(&path).unwrap();

  tracing::info!(target: "server","server initiated: listeng at {:?}", path);
  loop {
    let stream = listener.accept().await;
    tokio::spawn(async move {
      if let Ok((stream, addr)) = stream {
        tracing::info!(target: "server","connect established: {:?}", addr);

        let mut conn = life::server::Server::new(stream);

        tokio::spawn(async move {
          match conn.handle().await {
            Err(err) => {
              if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
                tracing::error!( target: "server", "c: {:?} {}", addr, err);
              }
            }

            _ => {
              tracing::warn!(target: "server", "connection satisfied: {addr:?}");
            }
          }
        });
      } else {
        tracing::error!("connect failed to establish: hard reset")
      }
    });
  }
}
