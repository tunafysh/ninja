use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use warp::{Filter, Reply, Rejection};
use log::{info, error};

use crate::manager::ServiceManager;
use crate::error::ServiceError;

// API Response types
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ServiceStartResponse {
    service_name: String,
    pid: u32,
    status: String,
}

#[derive(Serialize)]
struct ServiceStopResponse {
    service_name: String,
    status: String,
}

#[derive(Deserialize)]
struct ServiceActionRequest {
    service_name: String,
}

// Shared state type
type SharedServiceManager = Arc<Mutex<ServiceManager>>;

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// Convert ServiceError to HTTP status and message
impl warp::reject::Reject for ServiceError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (warp::http::StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if let Some(service_error) = err.find::<ServiceError>() {
        match service_error {
            ServiceError::ServiceNotFound(name) => (
                warp::http::StatusCode::NOT_FOUND,
                format!("Service '{}' not found", name),
            ),
            ServiceError::ConfigError(msg) => (
                warp::http::StatusCode::BAD_REQUEST,
                format!("Configuration error: {}", msg),
            ),
            ServiceError::SpawnFailed(service, _) => (
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to start service '{}'", service),
            ),
            ServiceError::NoPid => (
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get process ID".to_string(),
            ),
            _ => (
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        }
    } else if err.find::<warp::filters::body::BodyDeserializeError>().is_some() {
        (
            warp::http::StatusCode::BAD_REQUEST,
            "Invalid JSON body".to_string(),
        )
    } else {
        error!("Unhandled rejection: {:?}", err);
        (
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    };

    let json = warp::reply::json(&ApiResponse::<()>::error(message));
    Ok(warp::reply::with_status(json, code))
}

// Route handlers
async fn get_all_services(manager: SharedServiceManager) -> Result<impl Reply, Rejection> {
    match manager.lock().await.get_all_services().await {
        Ok(services) => Ok(warp::reply::json(&ApiResponse::success(services))),
        Err(e) => {
            error!("Failed to get all services: {:?}", e);
            Err(warp::reject::custom(e))
        }
    }
}

async fn get_running_services(manager: SharedServiceManager) -> Result<impl Reply, Rejection> {
    match manager.lock().await.get_running_services().await {
        Ok(services) => Ok(warp::reply::json(&ApiResponse::success(services))),
        Err(e) => {
            error!("Failed to get running services: {:?}", e);
            Err(warp::reject::custom(e))
        }
    }
}

async fn list_services(manager: SharedServiceManager) -> Result<impl Reply, Rejection> {
    let service_names = manager.lock().await.list_services();
    Ok(warp::reply::json(&ApiResponse::success(service_names)))
}

async fn get_service_status(
    service_name: String,
    manager: SharedServiceManager,
) -> Result<impl Reply, Rejection> {
    match manager.lock().await.get_service_status(&service_name) {
        Some(status) => Ok(warp::reply::json(&ApiResponse::success(status))),
        None => Err(warp::reject::custom(ServiceError::ServiceNotFound(
            service_name,
        ))),
    }
}

async fn start_service(
    request: ServiceActionRequest,
    manager: SharedServiceManager,
) -> Result<impl Reply, Rejection> {
    let service_name = request.service_name;
    
    match manager.lock().await.start_service(&service_name).await {
        Ok(pid) => {
            info!("Successfully started service: {}", service_name);
            let response = ServiceStartResponse {
                service_name,
                pid,
                status: "running".to_string(),
            };
            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to start service {}: {:?}", service_name, e);
            Err(warp::reject::custom(e))
        }
    }
}

async fn stop_service(
    request: ServiceActionRequest,
    manager: SharedServiceManager,
) -> Result<impl Reply, Rejection> {
    let service_name = request.service_name;
    
    match manager.lock().await.stop_service(&service_name).await {
        Ok(_) => {
            info!("Successfully stopped service: {}", service_name);
            let response = ServiceStopResponse {
                service_name,
                status: "stopped".to_string(),
            };
            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to stop service {}: {:?}", service_name, e);
            Err(warp::reject::custom(e))
        }
    }
}

async fn restart_service(
    request: ServiceActionRequest,
    manager: SharedServiceManager,
) -> Result<impl Reply, Rejection> {
    let service_name = request.service_name;
    
    // Stop the service first
    if let Err(e) = manager.lock().await.stop_service(&service_name).await {
        error!("Failed to stop service {} during restart: {:?}", service_name, e);
        return Err(warp::reject::custom(e));
    }
    
    // Wait a moment for cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Start the service
    match manager.lock().await.start_service(&service_name).await {
        Ok(pid) => {
            info!("Successfully restarted service: {}", service_name);
            let response = ServiceStartResponse {
                service_name,
                pid,
                status: "running".to_string(),
            };
            Ok(warp::reply::json(&ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to start service {} during restart: {:?}", service_name, e);
            Err(warp::reject::custom(e))
        }
    }
}

// Health check endpoint
async fn health_check() -> Result<impl Reply, Rejection> {
    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        timestamp: u64,
    }
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    
    Ok(warp::reply::json(&ApiResponse::success(response)))
}

// CORS configuration
fn with_cors() -> warp::filters::cors::Builder {
    warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
}

// Filter to inject shared manager
fn with_manager(
    manager: SharedServiceManager,
) -> impl Filter<Extract = (SharedServiceManager,), Error = Infallible> + Clone {
    warp::any().map(move || manager.clone())
}

pub async fn run_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(Mutex::new(ServiceManager::bootstrap()?));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Health check route
    let health = warp::path("health")
        .and(warp::get())
        .and_then(health_check);

    // Service management routes
    let services_base = warp::path("api").and(warp::path("services"));

    // GET /api/services - list all services with status
    let get_all = services_base
        .and(warp::get())
        .and(warp::path::end())
        .and(with_manager(manager.clone()))
        .and_then(get_all_services);

    // GET /api/services/running - get only running services
    let get_running = services_base
        .and(warp::path("running"))
        .and(warp::get())
        .and(warp::path::end())
        .and(with_manager(manager.clone()))
        .and_then(get_running_services);

    // GET /api/services/list - get service names only
    let list = services_base
        .and(warp::path("list"))
        .and(warp::get())
        .and(warp::path::end())
        .and(with_manager(manager.clone()))
        .and_then(list_services);

    // GET /api/services/{name}/status - get specific service status
    let get_status = services_base
        .and(warp::path::param::<String>())
        .and(warp::path("status"))
        .and(warp::get())
        .and(warp::path::end())
        .and(with_manager(manager.clone()))
        .and_then(get_service_status);

    // POST /api/services/start - start a service
    let start = services_base
        .and(warp::path("start"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_manager(manager.clone()))
        .and_then(start_service);

    // POST /api/services/stop - stop a service
    let stop = services_base
        .and(warp::path("stop"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_manager(manager.clone()))
        .and_then(stop_service);

    // POST /api/services/restart - restart a service
    let restart = services_base
        .and(warp::path("restart"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_manager(manager.clone()))
        .and_then(restart_service);

    // Combine all routes
    let routes = health
        .or(get_all)
        .or(get_running)
        .or(list)
        .or(get_status)
        .or(start)
        .or(stop)
        .or(restart)
        .with(with_cors())
        .recover(handle_rejection);

    info!("Kurokage API server started on {}", addr);
    info!("Available endpoints:");
    info!("  GET    /health");
    info!("  GET    /api/services");
    info!("  GET    /api/services/running");
    info!("  GET    /api/services/list");
    info!("  GET    /api/services/{{name}}/status");
    info!("  POST   /api/services/start");
    info!("  POST   /api/services/stop");
    info!("  POST   /api/services/restart");

    warp::serve(routes).run(addr).await;

    Ok(())
}