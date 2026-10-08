use std::{path::PathBuf, sync::Arc};

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use tokio::sync::Mutex;

use crate::{config::Config, switchbot::Switchbot};

struct AppState {
    client: Mutex<Option<Switchbot>>,
    config_directory: PathBuf,
}

pub fn router(config_directory: PathBuf, client: Option<Switchbot>) -> Router {
    Router::new()
        .route("/metrics", get(metrics).options(options))
        .with_state(Arc::new(AppState {
            client: Mutex::new(client),
            config_directory,
        }))
}

async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // 同時scrapeを直列化し、同じAPI/cacheをプロセス内で共有する。
    let mut client = state.client.lock().await;
    let result = async {
        if client.is_none() {
            *client = Some(Switchbot::new(Config::load(&state.config_directory)?)?);
        }
        client
            .as_mut()
            .expect("client initialized")
            .fetch_metrics()
            .await
    }
    .await;
    match result {
        Ok(metrics) => (
            StatusCode::OK,
            [("content-type", "text/plain; charset=utf-8")],
            metrics.render(),
        ),
        Err(error) => {
            eprintln!("metrics request failed: {error:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain; charset=utf-8")],
                "Internal Server Error".to_owned(),
            )
        }
    }
}

async fn options() -> impl IntoResponse {
    ([("allow", "GET, HEAD, OPTIONS")], "")
}

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Could not register SIGTERM handler");
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.expect("Could not register SIGINT handler"),
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("Could not register Ctrl-C handler");
}
