// #[tokio::main(flavor = "multi_thread")]
//
//

struct LogTime {
  inner: tokio::time::Instant,
}

impl LogTime {
  fn new() -> Self {
    Self {
      inner: tokio::time::Instant::now(),
    }
  }
}

impl tracing_subscriber::fmt::time::FormatTime for LogTime {
  fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
    let elapsed = self.inner.elapsed().as_secs();
    write!(w, "{}s", elapsed)
  }
}


#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .with_timer(LogTime::new())
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");

  if path.exists() {
    std::fs::remove_file(&path).unwrap();
  }

  let listener = tokio::net::UnixListener::bind(&path).unwrap();

  tracing::info!(target: "server","server initiated: listeng at {:?}", path);
  loop {
    let stream = listener.accept().await;
    tokio::spawn(async move {
      if let Ok((stream, addr)) = stream {
        tracing::info!(target: "server","*** connect established: {:?}", addr);

        let mut conn = life::server::Server::new(stream);

        tokio::spawn(async move {
          match conn.handle().await {
            Err(err) => {
              if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
                tracing::error!( target: "server", "*** connection exit, {}", err.to_string());
              }
            }

            _ => {
              tracing::warn!(target: "server", "*** connection exit");
            }
          }
        });
      } else {
        tracing::error!("??? connection failed")
      }
    });
  }
}
