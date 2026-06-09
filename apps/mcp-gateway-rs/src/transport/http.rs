//! HTTP front: MCP at /mcp, token-guarded control plane, /health open.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use serde_json::json;

use crate::config::McpConfig;
use crate::handler::Gateway;
use crate::GATEWAY_VERSION;

#[derive(Clone)]
struct AppState {
    gateway: Gateway,
    token: String,
}

pub fn router(gateway: Gateway, token: String) -> Router {
    let mcp_gateway = gateway.clone();
    let mcp = StreamableHttpService::new(
        move || Ok(mcp_gateway.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let state = AppState { gateway, token };

    // Everything except /health requires the bearer token (the MCP endpoint
    // and the control plane alike). route_layer applies to routes added before it.
    let protected = Router::new()
        .nest_service("/mcp", mcp)
        .route("/backends", get(list_backends))
        .route("/backends/{name}", get(get_backend))
        .route("/backends/{name}/restart", post(restart_backend))
        .route("/backends/{name}/stop", post(stop_backend))
        .route("/backends/{name}/start", post(start_backend))
        .route("/reload", post(reload))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_token,
        ));

    Router::new()
        .route("/health", get(health))
        .merge(protected)
        .with_state(state)
}

async fn require_token(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let ok = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        == Some(format!("Bearer {}", state.token).as_str());
    if !ok {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    next.run(req).await
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": GATEWAY_VERSION }))
}

async fn list_backends(State(state): State<AppState>) -> impl IntoResponse {
    let states = state.gateway.supervisor().states().await;
    let body: HashMap<String, String> = states
        .into_iter()
        .map(|(k, v)| (k, format!("{v:?}")))
        .collect();
    Json(json!({ "backends": body }))
}

async fn get_backend(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    match state.gateway.supervisor().state(&name).await {
        Some(s) => Json(json!({ "name": name, "state": format!("{s:?}") })).into_response(),
        None => (StatusCode::NOT_FOUND, "unknown backend").into_response(),
    }
}

async fn restart_backend(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.gateway.supervisor().restart(&name).await;
    Json(json!({ "restarted": name }))
}

async fn stop_backend(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.gateway.supervisor().stop(&name).await;
    Json(json!({ "stopped": name }))
}

async fn start_backend(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.gateway.supervisor().start_one(&name).await;
    Json(json!({ "started": name }))
}

async fn reload(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = match McpConfig::load() {
        Ok(c) => c,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("invalid config: {e}")).into_response(),
    };
    if let Err(e) = cfg.validate() {
        return (StatusCode::BAD_REQUEST, format!("invalid config: {e}")).into_response();
    }
    let report = state.gateway.supervisor().reload(cfg).await;
    Json(report).into_response()
}
