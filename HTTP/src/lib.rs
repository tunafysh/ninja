use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use ninja::manager::ShurikenManager;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

pub mod graphql;

#[derive(Serialize)]
struct ApiResponse<T>
where
    T: Serialize,
{
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

// Shared state for Axum
#[derive(Clone)]
struct AppState {
    manager: Arc<ShurikenManager>,
}

fn ok_response<T>(data: Option<T>) -> Response
where
    T: Serialize,
{
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data,
            error: None,
        }),
    )
        .into_response()
}

fn err_response(status: StatusCode, message: String) -> Response {
    (
        status,
        Json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message),
        }),
    )
        .into_response()
}

// Handler to start a shuriken
async fn start_shuriken(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Response {
    match state.manager.start(&name).await {
        Ok(()) => ok_response::<()>(None),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// Handler to stop a shuriken
async fn stop_shuriken(Path(name): Path<String>, State(state): State<AppState>) -> Response {
    match state.manager.stop(&name).await {
        Ok(()) => ok_response::<()>(None),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// List shuriken states
async fn list_shuriken_states(State(state): State<AppState>) -> Response {
    match state.manager.list(true).await {
        Ok(either) => {
            if let Some(left) = either.left() {
                let mut formatted = HashMap::new();
                for (name, value) in left.iter().cloned() {
                    formatted.insert(name, value);
                }

                ok_response(Some(formatted))
            } else {
                err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No shurikens found.".to_string(),
                )
            }
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// List shuriken names
async fn list_shurikens(State(state): State<AppState>) -> Response {
    match state.manager.list(false).await {
        Ok(either) => {
            if let Some(right) = either.right() {
                ok_response(Some(right))
            } else {
                err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No shurikens found.".to_string(),
                )
            }
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// Stop the API
async fn stop_api() -> StatusCode {
    std::process::exit(0);
}

// Main server function
pub async fn server(port: u16) -> Result<()> {
    let manager = Arc::new(ShurikenManager::new().await?);

    let app = Router::new()
        .route("/api/shurikens/start/{shuriken}", get(start_shuriken))
        .route("/api/shurikens/stop/{shuriken}", get(stop_shuriken))
        .route("/api/shurikens/list", get(list_shurikens))
        .route("/api/shurikens/list/states", get(list_shuriken_states))
        .route("/api/stop", get(stop_api))
        .with_state(AppState { manager });

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
