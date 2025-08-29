use crate::{manager::ShurikenManager, types::ShurikenState};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
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

#[get("/api/shurikens/list/states")]
async fn list_shuriken_states(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    
    let result = manager.list(true).await?.left();

    match result {
        
        Some(e) => {
            let mut formatted_data = HashMap::new();
            
            for item in e.iter() {
                let (name, value) = item.clone();

                formatted_data.insert(name, value);
            }
            
            Ok(HttpResponse::Ok().json(ApiResponse::<HashMap<String, ShurikenState>> {
                success: true,
                data: Some(formatted_data),
                error: None
        }))},
        None => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("No shurikens found.".to_string())
        }))
    }
} 

#[get("/api/shurikens/list")]
async fn list_shurikens(manager: web::Data<ShurikenManager>) -> Result<HttpResponse> {
    let result = manager.list(false).await?.right();
    match result {
        Some(value) => {
            Ok(HttpResponse::Ok().json(ApiResponse::<Vec<String>> {
              success: true,
              data: Some(value),
              error: None  
            }))
        },
        None => Ok(HttpResponse::InternalServerError().json( ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("No shurikens found.".to_string())
        }))
    }    
    
}

pub async fn server(port: u16) -> std::io::Result<()> {
    let manager = Arc::new(ShurikenManager::new().await.expect("Failed to create manager for web API"));
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