use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use tracing as log;


pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  log::info!(target: "rt", "async runtime is being initiated ...");

  let addr = std::net::SocketAddr::V4(SocketAddrV4::new( Ipv4Addr::new(127, 0, 0, 2), 8080));

  proxy_run(addr).await?;
  Ok(())
}




async fn proxy_run(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
  let listener = tokio::net::TcpListener::bind(addr).await?;
  log::info!("listening on tcp {}", addr);

  loop {
    if let Ok((_stream, peer)) = listener.accept().await {
      tokio::spawn(async move {
        log::info!("connected {}", peer);
      });
    }
  }
}

