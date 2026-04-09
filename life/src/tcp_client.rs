use std::net::Ipv4Addr;
use std::net::SocketAddr;

use life::client::client_handler;

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

  tracing_subscriber::fmt::fmt()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .init();

  let mut reconnect = false;

  tracing::info!(target: "client" ,"Tcp Client configured : {}", addr);
  loop {
    if reconnect {
      tracing::info!("Retrying in 2s");
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    reconnect = if let Ok(conn) = tokio::net::TcpStream::connect(&addr).await {
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
