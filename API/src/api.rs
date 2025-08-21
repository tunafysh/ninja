use crate::manager::ShurikenManager;
use serde::Serialize;
use actix_web::{get, web, App, HttpResponse, HttpServer, Result};

#[derive(Serialize)]
struct ApiResponse<T>
where 
    T: Serialize
{
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[get("/api/shurikens/start/{shuriken}")]
async fn start_shuriken(path: web::Path<String>, manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let name = path.into_inner();
    let result = manager.start(&name).await;
    match result {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                data: None,
                error: None
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string())
        }))
    }
} 

#[get("/api/shurikens/stop/{shuriken}")]
async fn stop_shuriken(path: web::Path<String>, manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let name = path.into_inner();
    let result = manager.stop(&name).await;
    match result {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                data: None,
                error: None
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string())
        }))
    }
} 

#[get("/api/shurikens/list/running")]
async fn list_running_shurikens(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    
    let result = manager.list(true).await;
    match result {
        Ok(e) => Ok(HttpResponse::Ok().json(ApiResponse::<Vec<String>> {
                success: true,
                data: Some(e.iter().map(|s| s.shuriken.name.clone()).collect()),
                error: None
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string())
        }))
    }
} 

#[get("/api/shurikens/list")]
async fn list_shurikens(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let result = manager.list(false).await;
    match result {
        Ok(e) => Ok(HttpResponse::Ok().json(ApiResponse::<Vec<String>> {
                success: true,
                data: Some(e.iter().map(|s| s.shuriken.name.clone()).collect()),
                error: None
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string())
        }))
    }
}

pub async fn server(port: u16) -> std::io::Result<()> {
    let manager = ShurikenManager::new().await.expect("Failed to create manager for web API");
    let manager_data = web::Data::new(manager);
    HttpServer::new(move || {
        App::new()
        .app_data(manager_data.clone())
        .service(start_shuriken)
        .service(stop_shuriken)
        .service(list_shurikens)
        .service(list_running_shurikens)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
   
}