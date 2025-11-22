use anyhow::Result;
use ninja::{manager::ShurikenManager, types::ShurikenState};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tide::http::mime;
use tide::prelude::*; // For JSON serialization
use tide::{Request, Response, StatusCode};

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

// Shared state for Tide
#[derive(Clone)]
struct AppState {
    manager: Arc<ShurikenManager>,
}

// Handler to start a shuriken
async fn start_shuriken(req: Request<AppState>) -> tide::Result {
    let name: String = req.param("shuriken")?.to_string();
    let state = req.state();

    match state.manager.start(&name).await {
        Ok(()) => Ok(Response::builder(StatusCode::Ok)
            .body(json!(&ApiResponse::<()> {
                success: true,
                data: None,
                error: None
            }))
            .content_type(mime::JSON)
            .build()),
        Err(e) => Ok(Response::builder(StatusCode::InternalServerError)
            .body(json!(&ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string())
            }))
            .content_type(mime::JSON)
            .build()),
    }
}

// Handler to stop a shuriken
async fn stop_shuriken(req: Request<AppState>) -> tide::Result {
    let name: String = req.param("shuriken")?.to_string();
    let state = req.state();

    match state.manager.stop(&name).await {
        Ok(()) => Ok(Response::builder(StatusCode::Ok)
            .body(json!(&ApiResponse::<()> {
                success: true,
                data: None,
                error: None
            }))
            .content_type(mime::JSON)
            .build()),
        Err(e) => Ok(Response::builder(StatusCode::InternalServerError)
            .body(json!(&ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string())
            }))
            .content_type(mime::JSON)
            .build()),
    }
}

// List shuriken states
async fn list_shuriken_states(req: Request<AppState>) -> tide::Result {
    let state = req.state();

    match state.manager.list(true).await {
        Ok(either) => {
            if let Some(left) = either.left() {
                let mut formatted = HashMap::new();
                for (name, value) in left.iter().cloned() {
                    formatted.insert(name, value);
                }

                Ok(Response::builder(StatusCode::Ok)
                    .body(json!(&ApiResponse::<HashMap<String, ShurikenState>> {
                        success: true,
                        data: Some(formatted),
                        error: None
                    }))
                    .content_type(mime::JSON)
                    .build())
            } else {
                Ok(Response::builder(StatusCode::InternalServerError)
                    .body(json!(&ApiResponse::<()> {
                        success: false,
                        data: None,
                        error: Some("No shurikens found.".to_string())
                    }))
                    .content_type(mime::JSON)
                    .build())
            }
        }
        Err(e) => Ok(Response::builder(StatusCode::InternalServerError)
            .body(json!(&ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string())
            }))
            .content_type(mime::JSON)
            .build()),
    }
}

// List shuriken names
async fn list_shurikens(req: Request<AppState>) -> tide::Result {
    let state = req.state();

    match state.manager.list(false).await {
        Ok(either) => {
            if let Some(right) = either.right() {
                Ok(Response::builder(StatusCode::Ok)
                    .body(json!(&ApiResponse {
                        success: true,
                        data: Some(right),
                        error: None
                    }))
                    .content_type(mime::JSON)
                    .build())
            } else {
                Ok(Response::builder(StatusCode::InternalServerError)
                    .body(json!(&ApiResponse::<Vec<String>> {
                        success: false,
                        data: None,
                        error: Some("No shurikens found.".to_string())
                    }))
                    .content_type(mime::JSON)
                    .build())
            }
        }
        Err(e) => Ok(Response::builder(StatusCode::InternalServerError)
            .body(json!(&ApiResponse::<Vec<String>> {
                success: false,
                data: None,
                error: Some(e.to_string())
            }))
            .content_type(mime::JSON)
            .build()),
    }
}

// Stop the API
async fn stop_api(_req: Request<AppState>) -> tide::Result {
    std::process::exit(0);
}

// Main server function
pub async fn server(port: u16) -> Result<()> {
    let manager = Arc::new(ShurikenManager::new().await?);

    let mut app = tide::with_state(AppState { manager });

    app.at("/api/shurikens/start/:shuriken").get(start_shuriken);
    app.at("/api/shurikens/stop/:shuriken").get(stop_shuriken);
    app.at("/api/shurikens/list").get(list_shurikens);
    app.at("/api/shurikens/list/states")
        .get(list_shuriken_states);
    app.at("/api/stop").get(stop_api);
    app.listen(format!("127.0.0.1:{}", port)).await?;
    Ok(())
}
