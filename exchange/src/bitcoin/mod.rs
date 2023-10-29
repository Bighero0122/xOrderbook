use std::convert::Infallible;

mod client;
pub use client::BitcoinRpcClient;

mod rpc;

use crate::{signal::Signals, Config};

pub mod proto {
    tonic::include_proto!("bitcoincore");
}

struct BitcoinCoreRpcImpl {
    config: Config,
    signals: Signals,
}

impl proto::bitcoin_core_rpc_server::BitcoinCoreRpc for BitcoinCoreRpcImpl {}

pub async fn start_grpc_proxy(
    config: Config,
    signals: Signals,
) -> Result<(), tonic::transport::Error> {
    use proto::bitcoin_core_rpc_server::BitcoinCoreRpcServer;

    let addr = config.bitcoin_grpc_bind_addr();
    tracing::info!(%addr, "starting grpc proxy");

    let svc = BitcoinCoreRpcServer::new(BitcoinCoreRpcImpl { config, signals });

    tonic::transport::Server::builder()
        .add_service(svc)
        .serve_with_shutdown(addr, async move {
            let _ = signals.ctrl_c().await;
            tracing::warn!("SIGINT received");
        })
        .await
}

/// Connect to a Bitcoin Core RPC server using the given configuration.
///
/// This function will return a client that can be used to make RPC calls to the
/// Bitcoin Core server and depending on the configuration, it will either use
/// jsonrpc over a direct http connection to the bitcoincore node or it will connect to the grpc proxy.
///
pub async fn connect_bitcoin_rpc(config: &Config) -> Result<BitcoinRpcClient, Infallible> {
    Ok(
        BitcoinRpcClient::new_grpc(config.bitcoin_grpc_endpoint().clone())
            .await
            .unwrap(),
    )
}
