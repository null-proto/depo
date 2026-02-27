#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

#[allow(unused)]
struct TimeSample {
  inner: tokio::time::Instant,
}

#[allow(unused)]
impl TimeSample {
  fn new() -> Self {
    Self {
      inner: tokio::time::Instant::now(),
    }
  }
}

#[allow(unused)]
impl tracing_subscriber::fmt::time::FormatTime for TimeSample {
  fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
    let elapsed = self.inner.elapsed().as_secs();
    write!(w, "{}s", elapsed)
  }
}

#[cfg(debug_assertions)]
fn init_tracing() {
  use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};

  let tracing_filter = tracing_subscriber::filter::Targets::new()
    .with_target("rt", tracing::Level::TRACE)
    .with_target("main", tracing::Level::TRACE)
    .with_target("net", tracing::Level::TRACE)
    .with_target("write", tracing::Level::TRACE)
    .with_default(tracing::Level::ERROR);

  tracing_subscriber::registry()
    .with(
      tracing_subscriber::fmt::layer()
        .with_level(true)
        .with_target(true)
        .with_timer(TimeSample::new())
        .with_thread_names(true),
    )
    .with(tracing_filter)
    .init();
}

#[cfg(not(debug_assertions))]
fn init_tracing() {
  use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};

  let tracing_filter = tracing_subscriber::filter::Targets::new()
    .with_target("rt", tracing::Level::INFO)
    .with_target("main", tracing::Level::INFO)
    .with_target("net", tracing::Level::INFO)
    .with_target("write", tracing::Level::INFO)
    .with_default(tracing::Level::WARN);

  tracing_subscriber::registry()
    .with(
      tracing_subscriber::fmt::layer()
        .with_level(true)
        .without_time()
        .with_target(true),
    )
    .with(tracing_filter)
    .init();
}

fn main() {
  init_tracing();
  trace!(target: "main", "initiated ...");

  if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
  {
    if let Err(err) = rt.block_on(main_()) {
      error!(target: "main" ,"runtime returns error ...");
      if let Some(err) = err.downcast_ref::<tokio::io::Error>() {
        error!(target: "main" ,"{} {}" ,err.kind() , err.to_string() );
      } else {
        error!(target: "main" ,"{}" ,err);
      }
    } else {
    }
  } else {
    error!(target: "main" ,"canont crete runtime ...");
    info!(target: "main", "exiting ...");
    std::process::exit(1);
  }
  info!(target: "main", "exiting ...");
}

async fn main_() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  info!(target: "rt", "initiated ...");

  Ok(())
}

