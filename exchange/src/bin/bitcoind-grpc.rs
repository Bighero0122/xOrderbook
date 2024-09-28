use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let body = async {
        tracing_subscriber::fmt::fmt()
            .with_file(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();

        let config = exchange::Configuration::load_from_path(
            exchange::config::config_file_path().unwrap().as_path(),
        )?;
        exchange::bitcoin::start_grpc_proxy(config, exchange::signal::from_host_os())
            .await
            .map_err(|err| Box::new(err) as Box<_>)
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let res = runtime.block_on(body);
    runtime.shutdown_timeout(Duration::from_secs(2));
    res
}
