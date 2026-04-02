use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

// #[tokio::main(flavor = "multi_thread")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .init();

  let path = std::path::PathBuf::from("/tmp/sock.0");
  let s_path = path.clone();

  let s = tokio::spawn(async move {
    server(&s_path).await;
  });

  let c = tokio::spawn(async move {
    client(&path).await;
  });

  _ = tokio::join!(s, c);
}

async fn server(path: &std::path::Path) {
  let listener = tokio::net::UnixListener::bind(path).unwrap();

  tracing::info!(target: "server","server initiated: listeng at {:?}", path);
  loop {
    let stream = listener.accept().await;
    tokio::spawn(async move {
      if let Ok((mut stream, addr)) = stream {
        tracing::info!(target: "server","connect established: {:?}", addr);

        // let (mut read, write) = stream.into_split();
        match server_heldler(&mut stream).await {
          Ok(_) => {}
          Err(_) => {}
        }

        // let read_thread = tokio::spawn(async move {
        //   tracing::debug!(target: "server","connect established: setup reader");
        //   loop {
        //     match server_read_handler(&mut read).await {
        //       Err(err) => {
        //         if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
        //           tracing::error!( target: "server", "c: {:?} {}", addr, err);
        //         }
        //       }
        //
        //       _ => {
        //         tracing::warn!(target: "server", "connection satisfied: {addr:?}");
        //       }
        //     }
        //   }
        // });
        //
        // let write_thread = tokio::spawn(async move {
        //   let _w = write;
        //   tracing::debug!(target: "server","connect established: setup writer");
        // });
        //
        // _ = tokio::join!(read_thread, write_thread);
      } else {
        tracing::error!("connect failed to establish: hard reset")
      }
    });
  }
}

async fn server_heldler(
  mut stream: &mut tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  loop {
    match server_read_handler(&mut stream).await {
      Err(err) => {
        if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
          tracing::error!( target: "server", "c: {:?} {}", stream.peer_addr(), err);
        }
      }

      _ => {
        tracing::warn!(target: "server", "connection satisfied: {:?}", stream.peer_addr());
      }
    }
  }
}

#[allow(unreachable_code)]
async fn server_read_handler(
  stream: &mut tokio::net::UnixStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let mut buf = Box::pin(tokio::io::BufReader::new(stream));
  let mut sbuf = String::new();

  loop {
    buf.read_line(&mut sbuf).await?;

    tracing::info!(target: "server" , "ingress: {:?}", sbuf.trim_end());
    sbuf.truncate(0);
  }

  Ok(())
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

async fn client(path: &std::path::Path) {
  tokio::time::sleep(std::time::Duration::from_secs(1)).await;

  if let Ok(conn) = tokio::net::UnixStream::connect(path).await {
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

