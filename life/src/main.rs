fn main() {

  tracing_subscriber::fmt().init();
  tracing::info!("--");

  let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("Cannot create rt");

  let res = rt.block_on(async { life::async_::main().await });

  tracing::info!("main return with : {:#?}", res);
}
