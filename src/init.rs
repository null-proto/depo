use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use tokio::net::TcpStream;
use tracing as log;


pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  log::info!(target: "rt", "async runtime is being initiated ...");

  let addr = std::net::SocketAddr::V4(SocketAddrV4::new( Ipv4Addr::new(127, 0, 0, 2), 8080));

  runner(addr).await?;
  Ok(())
}


async fn runner(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{

  let listener = tokio::net::TcpListener::bind(addr).await?;
  log::info!("listening on tcp {}", addr);

  loop {
    if let Ok((stream, peer)) = listener.accept().await {
      tokio::spawn( handler(stream, peer) );
    }
  }
}

async fn handler(stream : TcpStream , peer : SocketAddr)-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  use h2::server::handshake;

  log::info!("connected {}", peer);
  let mut conn = handshake(stream).await?;


  let (conn , _sr ) =  conn.accept().await.expect("None found").expect("no valid request");

  log::trace!("received {:?}" , conn);

  Ok(())
}
