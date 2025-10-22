use actix_web::{App, HttpResponse, HttpServer, Result, get, web};
use ninja::{manager::ShurikenManager, types::ShurikenState};
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

#[get("/api/shurikens/start/{shuriken}")]
async fn start_shuriken(
    path: web::Path<String>,
    manager: web::Data<ShurikenManager>,
) -> Result<HttpResponse> {
    let name = path.into_inner();
    let result = manager.start(&name).await;
    match result {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            data: None,
            error: None,
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

#[get("/api/shurikens/stop/{shuriken}")]
async fn stop_shuriken(
    path: web::Path<String>,
    manager: web::Data<ShurikenManager>,
) -> Result<HttpResponse> {
    let name = path.into_inner();
    let result = manager.stop(&name).await;
    match result {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            data: None,
            error: None,
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

#[get("/api/shurikens/list/states")]
async fn list_shuriken_states(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let result = manager
        .list(true)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .left();

    match result {
        Some(e) => {
            let mut formatted_data = HashMap::new();

            for item in e.iter() {
                let (name, value) = item.clone();

                formatted_data.insert(name, value);
            }

            Ok(
                HttpResponse::Ok().json(ApiResponse::<HashMap<String, ShurikenState>> {
                    success: true,
                    data: Some(formatted_data),
                    error: None,
                }),
            )
        }
        None => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("No shurikens found.".to_string()),
        })),
    }
}

#[get("/api/shurikens/list")]
async fn list_shurikens(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let result = manager
        .list(false)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .right();
    match result {
        Some(value) => Ok(HttpResponse::Ok().json(ApiResponse::<Vec<String>> {
            success: true,
            data: Some(value),
            error: None,
        })),
        None => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("No shurikens found.".to_string()),
        })),
    }
}

#[get("/api/stop")]
async fn stop_api() -> HttpResponse {
    // Spawn a task so we can respond before exiting
    // also if you say this is not graceful then i give you full permission to bang your head on your desk
    // to come up with a solution for this. I hate handles.
    tokio::spawn(async {
        std::process::exit(0);
    });

    HttpResponse::Ok().body("Exiting immediately")
}

pub async fn server(port: u16) -> std::io::Result<()> {
    let manager = Arc::new(
        ShurikenManager::new()
            .await
            .expect("Failed to create manager for web API"),
    );
    let manager_data = web::Data::new(manager);
    HttpServer::new(move || {
        App::new()
            .app_data(manager_data.clone())
            .service(start_shuriken)
            .service(stop_shuriken)
            .service(list_shurikens)
            .service(list_shuriken_states)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
