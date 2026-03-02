use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use tokio::net::TcpStream;
use tracing as log;

pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  log::info!(target: "rt", "async runtime is being initiated ...");

  let addr = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 2), 8080));

  runner(addr).await?;
  Ok(())
}

async fn runner(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let listener = tokio::net::TcpListener::bind(addr).await?;
  log::info!("listening on tcp {}", addr);

  loop {
    if let Ok((stream, peer)) = listener.accept().await {
      tokio::spawn(async move {
        if let Err(e) = handler(stream, peer).await {
          if let Some(e) = e.downcast_ref::<h2::Error>() {
            log::error!("{:?}", e);
            log::debug!("{}", e);
          } else if let Some(e) = e.downcast_ref::<std::io::Error>() {
            log::error!("{:?}", e);
            log::debug!("{}", e);
          }
        }
      });
    }
  }
}

async fn handler(
  stream: TcpStream,
  peer: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  use h2::server::handshake;

  log::info!("connected {}", peer);
  let mut conn = handshake(stream).await?;

  let (mut conn, mut _sr) = conn
    .accept()
    .await
    .expect("None found")
    .expect("no valid request");

  log::trace!("header > {:?}", conn);
  if let Some(Ok(body)) = conn.body_mut().data().await {
    log::trace!("body > {:?}", body);
    log::trace!(" *** ");
  } else {
    log::trace!(" *** ");
  }

  _sr.send_response(
    http::Response::builder().status(http::StatusCode::OK).body(())?,
    true,
  )?;

  Ok(())
}
