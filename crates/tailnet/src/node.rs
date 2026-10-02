use std::{
    convert::Infallible,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use anyhow::{Context, bail};
use serde::Serialize;
use tailscale::{AuthState, Config, Device, NodeInfo};
use tokio::{sync::watch, time};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use url::Url;

use crate::{ActiveConfig, config::AuthKey};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    Connecting,
    CheckingAuthorization,
    Approved,
    AwaitingApproval {
        approve_at: Url,
    },
    OpeningPorts {
        #[serde(flatten)]
        node: PublicNodeInfo,
    },
    Reachable {
        #[serde(flatten)]
        node: PublicNodeInfo,
    },
    ShuttingDown,
    Exiting,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicNodeInfo {
    pub machine_name: String,
    pub full_domain: String,
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
    pub tags: Vec<String>,
}

impl From<&NodeInfo> for PublicNodeInfo {
    fn from(node: &NodeInfo) -> Self {
        Self {
            machine_name: node.hostname.clone(),
            full_domain: node.fqdn(false),
            ipv4: node.tailnet_address.ipv4.addr(),
            ipv6: node.tailnet_address.ipv6.addr(),
            tags: node.tags.clone(),
        }
    }
}

fn device_args(active: &ActiveConfig) -> (Config, Option<AuthKey>) {
    let device = Config {
        key_state: active.stored.persist_state.clone(),
        control_server_url: active.stored.preferences.control_server_url.clone(),
        client_name: Some(env!("CARGO_PKG_NAME").to_string()),
        requested_hostname: Some(active.stored.preferences.requested_hostname.clone()),
        requested_tags: active.stored.preferences.requested_tags.clone(),
        ephemeral: active.stored.preferences.ephemeral,
    };
    let auth_key = active.auth_key.clone();
    (device, auth_key)
}

const fn local(port: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
}

async fn is_authorized(device: &Device) -> AuthState {
    let mut backoff = Duration::from_secs(1);
    loop {
        match device.is_authorized().await {
            Ok(state) => return state,
            Err(e) => warn!(?e, "Could not report whether this node has been authorized"),
        }
        time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(900));
    }
}

async fn self_node(device: &Device) -> NodeInfo {
    let mut backoff = Duration::from_secs(1);
    loop {
        match device.self_node().await {
            Ok(node) => return node,
            Err(e) => warn!(?e, "Could not get our node info"),
        }
        time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(900));
    }
}

async fn proxy_connection(mut source: tailscale::netstack::TcpStream, target: SocketAddr) {
    let mut target = match tokio::net::TcpStream::connect(target).await {
        Ok(t) => t,
        Err(e) => {
            warn!(?e, "could not open connection");
            return;
        }
    };

    match tokio::io::copy_bidirectional(&mut source, &mut target).await {
        Ok((inbound, outbound)) => debug!(?inbound, ?outbound, "connection ended"),
        Err(e) => warn!(?e, "connection failed"),
    }
}

async fn forward_port(
    listener: tailscale::netstack::TcpListener,
    target: SocketAddr,
) -> anyhow::Result<Infallible> {
    loop {
        let source = listener
            .accept()
            .await
            .context("could not accept connection")?;
        tokio::spawn(proxy_connection(source, target));
    }
}

async fn connect_and_serve(
    device: &Device,
    status: &watch::Sender<Status>,
) -> anyhow::Result<Infallible> {
    info!("Checking authorization");
    status.send_replace(Status::CheckingAuthorization);

    match is_authorized(device).await {
        AuthState::Authorized => {
            info!("Control has authorized this node");
            status.send_replace(Status::Approved);
        }
        AuthState::NotAuthorized(url) => {
            info!("Control has not authorized this node yet, it can be approved at {url}");
            status.send_replace(Status::AwaitingApproval { approve_at: url });
        }
    }

    let node = PublicNodeInfo::from(&self_node(device).await);

    info!(?node.machine_name, "Opening ports");
    status.send_replace(Status::OpeningPorts { node: node.clone() });

    let (listener_80, listener_443, listener_22) = tokio::try_join!(
        device.tcp_listen((node.ipv4, 80).into()),
        device.tcp_listen((node.ipv4, 443).into()),
        device.tcp_listen((node.ipv4, 22).into()),
    )
    .context("could not bind listener")?;

    info!("Reachable");
    status.send_replace(Status::Reachable { node });

    let never = tokio::select! {
        r = forward_port(listener_80, local(80)) => r.context("HTTP proxy failed"),
        r = forward_port(listener_443, local(443)) => r.context("HTTPS proxy failed"),
        r = forward_port(listener_22, local(22)) => r.context("SSH proxy failed"),
    }?;
    match never {};
}

pub async fn connect_and_serve_forever(
    mut active_config: watch::Receiver<ActiveConfig>,
    status: &watch::Sender<Status>,
    stop: CancellationToken,
) -> anyhow::Result<()> {
    while !stop.is_cancelled() {
        status.send_replace(Status::Connecting);

        let (device_config, auth_key) = device_args(&active_config.borrow_and_update());

        let device = Device::new(&device_config, auth_key.map(|key| key.into_exposed())).await?;

        let () = tokio::select! {
            biased;
            () = stop.cancelled() => (),
            r = active_config.changed() => r.context("config watch channel closed")?,
            r = connect_and_serve(&device, status) => r.context("connect and serve failed").map(|never|match never{})?,
        };

        info!("Shutting down");
        status.send_replace(Status::ShuttingDown);

        if !device.shutdown(Some(Duration::from_secs(10))).await {
            bail!("could not stop the device")
        }
    }
    Ok(())
}

pub async fn run(
    active_config: watch::Receiver<ActiveConfig>,
    status: watch::Sender<Status>,
    stop: CancellationToken,
) -> anyhow::Result<()> {
    let r = connect_and_serve_forever(active_config, &status, stop).await;
    info!("Exiting");
    status.send_replace(Status::Exiting);
    r
}
