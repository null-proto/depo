use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing as log;

use crate::http1;

pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  log::info!(target: "rt", "async runtime is being initiated ...");

  let addr = std::net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 2), 8080));

  runner(addr).await?;
  Ok(())
}

async fn runner(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let listener = tokio::net::TcpListener::bind(addr).await?;
  log::info!("listening on tcp {}", addr);

  let tls_cfg = tls_config()?;

  loop {
    if let Ok((stream, peer)) = listener.accept().await {
      let tls_cfg = tls_cfg.clone();
      tokio::spawn(async move {
        if let Err(e) = handler(stream, peer, tls_cfg).await {
          if let Some(e) = e.downcast_ref::<h2::Error>() {
            log::error!(" h2:] {}", e);
            log::debug!("{:?}", e);
          } else if let Some(e) = e.downcast_ref::<std::io::Error>() {
            log::error!("std:] {}: {}", e.kind(), e.to_string());
          } else {
            log::error!("unk:] {:?}", e);
          }
        }
      });
    }
  }
}

async fn handler(
  stream: TcpStream,
  peer: SocketAddr,
  tls_cfg: TlsAcceptor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  log::info!("connected {}", peer);

  let stream = tls_cfg.accept(stream).await?;
  log::debug!("tls handshake completed {}", peer);

  let protocol = stream.get_ref().1.alpn_protocol();

  match protocol {
    Some(b"h2") => h2_handler(stream).await?,
    Some(b"http/1.1") => {
      log::warn!("experimental protocol implementation in use: http/1.1");
      http1::h1_handler(stream).await?;
    }
    Some(u) => {
      log::error!("unsupported protocol: {}", String::from_utf8_lossy(u));
    }
    None => {
      log::error!("protocol not defined");
    }
  }

  Ok(())
}

async fn h2_handler(
  stream: TlsStream<TcpStream>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  // use h2::server::handshake;
  // use h2::server::Connection;
  // use h2::server::Handshake;
  use h2::server::Builder;

  let mut conn = Box::pin(
    Builder::new()
      .enable_connect_protocol()
      .handshake::<_, bytes::Bytes>(stream),
  )
  .await?;
  // let mut conn = handshake(stream).await?;

  while let Some(parts) = conn.accept().await {
    let (mut res, mut sr) = parts?;

    log::trace!("header > {:?}", res);
    if let Some(Ok(body)) = res.body_mut().data().await {
      log::trace!("body > {:?}", body);
      log::trace!(" ***");
    } else {
      log::trace!(" ***");
    }

    // _sr.send_reset(Reason::HTTP_1_1_REQUIRED);

    // tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
    sr.send_response(
      http::Response::builder()
        .status(http::StatusCode::NOT_FOUND)
        .body(())?,
      true,
    )?;
  }
  log::trace!(" *** connection ended");

  Ok(())
}

fn tls_config() -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
  use rustls::ServerConfig;
  use rustls::pki_types::{CertificateDer, PrivateKeyDer};
  use std::{fs::File, io::BufReader, sync::Arc};
  use tokio_rustls::TlsAcceptor;

  let certs = {
    let mut reader = BufReader::new(File::open("cert.pem")?);
    rustls_pemfile::certs(&mut reader)
      .map(|c| Some(CertificateDer::from(c.ok()?)))
      .flatten()
      .collect::<Vec<_>>()
  };

  let key = {
    let mut reader = BufReader::new(File::open("key.pem")?);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader).collect::<Result<Vec<_>, _>>()?;
    PrivateKeyDer::from(keys[0].clone_key())
  };

  let mut config = ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(certs, key)?;

  config.alpn_protocols = vec![
    b"h2".to_vec(),
   b"http/1.1".to_vec()
  ];

  Ok(TlsAcceptor::from(Arc::new(config)))
}
