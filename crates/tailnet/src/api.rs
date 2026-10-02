use anyhow::Context;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use log::info;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::{
    ActiveConfig,
    config::{AuthKey, Preferences, StoredConfig},
    node::Status,
};

#[derive(Clone)]
struct AppState {
    config: watch::Sender<ActiveConfig>,
    status: watch::Receiver<Status>,
}

async fn get_status(State(api): State<AppState>) -> Json<Status> {
    Json(api.status.borrow().clone())
}

async fn get_preferences(State(api): State<AppState>) -> Json<Preferences> {
    Json(api.config.borrow().stored.preferences.clone())
}

#[derive(serde::Deserialize)]
struct ReconnectBody {
    #[serde(flatten)]
    preferences: Preferences,
    auth_key: Option<String>,
}

async fn post_reconnect(
    State(api): State<AppState>,
    Json(body): Json<ReconnectBody>,
) -> Result<impl IntoResponse, StatusCode> {
    let auth_key = body
        .auth_key
        .filter(|key| !key.is_empty())
        .map(AuthKey::new);

    let persist_state = api.config.borrow().stored.persist_state.clone();

    let stored = StoredConfig {
        preferences: body.preferences.clone(),
        persist_state: persist_state.clone(),
    };

    match stored.write() {
        Ok(()) => (),
        Err(e) => {
            error!(?e, "could not write the stored config");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    api.config.send_replace(ActiveConfig { stored, auth_key });

    Ok(StatusCode::ACCEPTED)
}

fn router(api: AppState) -> Router {
    let router = Router::new()
        .route(
            concat!("/local/", env!("CARGO_PKG_NAME"), "/api/reconnect"),
            post(post_reconnect),
        )
        .route(
            concat!("/local/", env!("CARGO_PKG_NAME"), "/api/status"),
            get(get_status),
        )
        .route(
            concat!("/local/", env!("CARGO_PKG_NAME"), "/api/preferences"),
            get(get_preferences),
        );

    #[cfg(any(target_arch = "x86_64", target_os = "macos"))]
    let router = {
        use tower_http::services::ServeDir;
        router.nest_service(
            concat!("/local/", env!("CARGO_PKG_NAME")),
            ServeDir::new("html"),
        )
    };

    router.with_state(api)
}

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
async fn tcp_listener() -> anyhow::Result<(&'static str, tokio::net::TcpListener)> {
    const ADDR: &str = "127.0.0.1:2001";
    let listener = tokio::net::TcpListener::bind(ADDR).await?;
    Ok((ADDR, listener))
}

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
fn uds_listener() -> anyhow::Result<(&'static str, tokio::net::UnixListener)> {
    use std::os::unix::fs::PermissionsExt;

    use tracing::debug;

    const UDS_PATH: &str = env!("UDS_PATH");

    match std::fs::remove_file(UDS_PATH) {
        Ok(()) => debug!("Removed old socket"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => debug!("No old socket to remove"),
        Err(e) => return Err(e).context("Failed to remove old socket"),
    }
    let listener = tokio::net::UnixListener::bind(UDS_PATH).context("could not bind to socket")?;

    std::fs::set_permissions(UDS_PATH, std::fs::Permissions::from_mode(0o666))
        .context("could not set permissions on socket")?;

    Ok((UDS_PATH, listener))
}

pub async fn run(
    config: watch::Sender<ActiveConfig>,
    published: watch::Receiver<Status>,
    stop: CancellationToken,
) -> anyhow::Result<()> {
    #[cfg(any(target_arch = "x86_64", target_os = "macos"))]
    let (target, listener) = tcp_listener()
        .await
        .context("could not create bind TCP listener")?;

    #[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
    let (target, listener) = uds_listener().context("could not create bind UDS listener")?;

    info!("Management API listening on {target}");

    axum::serve(
        listener,
        router(AppState {
            config,
            status: published,
        }),
    )
    .with_graceful_shutdown(stop.cancelled_owned())
    .await?;

    info!("Management API stopped");

    Ok(())
}
