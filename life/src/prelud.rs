pub type Signel = Result<crate::ret::Return, Box<dyn std::error::Error + Send + Sync + 'static>>;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader};

pub use crate::ret::Return;

pub async fn readln() -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
  use tokio::io::AsyncWrite;
  use tokio::io::AsyncWriteExt;

  tracing::info!( target: "input" , "access stdin :");


  let mut stdout = tokio::io::stdout();

  stdout.write(b"input > ").await?;
  stdout.flush().await?;

  let mut s = String::new();
  let mut reader = BufReader::new(tokio::io::stdin());

  reader.read_line(&mut s).await?;

  Ok(s.trim().to_owned())
}
