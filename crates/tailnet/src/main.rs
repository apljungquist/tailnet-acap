#![forbid(unsafe_code)]
mod api;
mod config;
mod node;

use anyhow::Context;
use log::{LevelFilter, error, info};
use nix::sys::stat::Mode;
use tokio::{
    signal::unix::{Signal, SignalKind, signal},
    sync::watch,
};
use tokio_util::sync::CancellationToken;

use crate::config::{AuthKey, StoredConfig};

async fn watch_signals(mut terminate: Signal, mut interrupt: Signal, stop: CancellationToken) {
    tokio::select! {
        _ = terminate.recv() => info!("Terminated"),
        _ = interrupt.recv() => info!("Interrupted"),
    }
    stop.cancel();
}

struct ActiveConfig {
    stored: StoredConfig,
    auth_key: Option<AuthKey>,
}

#[expect(
    clippy::unwrap_used,
    reason = "should panic: the program is not designed to run in environment where these fail"
)]
async fn run() -> anyhow::Result<()> {
    let terminate = signal(SignalKind::terminate()).unwrap();
    let interrupt = signal(SignalKind::interrupt()).unwrap();
    let persistent = StoredConfig::ensure().context("could not create the default config")?;

    let (tx_config, rx_config) = watch::channel(ActiveConfig {
        stored: persistent,
        auth_key: None,
    });
    let (tx_status, rx_status) = watch::channel(node::Status::Connecting);
    let stop = CancellationToken::new();

    tokio::spawn(watch_signals(terminate, interrupt, stop.clone()));

    let ((), ()) = tokio::try_join!(
        api::run(tx_config, rx_status, stop.clone()),
        node::run(rx_config, tx_status, stop),
    )?;
    Ok(())
}

#[expect(
    clippy::unwrap_used,
    reason = "should panic: the program is not designed to run in environment where these fail"
)]
fn main() {
    acap_logging::Builder::new()
        .module_level("tailscale", LevelFilter::Warn)
        .module_level("ts_", LevelFilter::Warn)
        .module_level("kameo", LevelFilter::Warn)
        .module_level("tracing::span", LevelFilter::Warn)
        .init();

    std::panic::set_hook(Box::new(|info| error!("Panicked: {info}")));

    nix::sys::stat::umask(Mode::S_IRWXG.union(Mode::S_IRWXO));

    let Ok(()) = rustls_openssl::default_provider().install_default() else {
        panic!("could not install our preferred crypto provider")
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(run()).unwrap();
}
