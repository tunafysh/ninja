use ninja::{manager::ShurikenManager, shuriken::Shuriken};

#[tauri::command]
pub async fn start_shuriken(name: &str) -> Result<(), String> {
    log::info!("Starting service...");
    let service_manager = ShurikenManager::new().await.map_err(|e| e.to_string())?;
    match service_manager.start(name).await {
        Ok(_) => {
            log::info!("Service started successfully.");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to start service: {}", e);
            Err(format!("Failed to start service: {}", e))
        }
    }
}

#[tauri::command]
pub async fn stop_shuriken(name: &str) -> Result<(), String> {
    log::info!("Stopping service...");
    let service_manager = ShurikenManager::new().await.map_err(|e| e.to_string())?;
    match service_manager.stop(name).await {
        Ok(_) => {
            log::info!("Service stopped successfully.");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to stop service: {}", e);
            Err(format!("Failed to stop service: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_all_shurikens() -> Result<Vec<Shuriken>, String> {
    log::info!("Retrieving all services...");
    let manager = ShurikenManager::new().await.map_err(|e| e.to_string())?;
    Ok(manager.list(false).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn get_running_shurikens() -> Result<Vec<Shuriken>, String> {
    log::info!("Retrieving running services...");
    let service_manager = ShurikenManager::new().await.map_err(|e| e.to_string())?;
    match service_manager.list(true).await {
        Ok(services) => {
            log::info!("Retrieved running services successfully.");
            Ok(Vec::from_iter(services.into_iter().map(|s| s)))
        }
        Err(e) => {
            log::error!("Failed to retrieve running services: {}", e);
            Err(format!("Failed to retrieve running services: {}", e))
        }
    }
}

#[tauri::command]
pub fn enable_mcp(transport: &str) -> Result<(), String> {
    log::info!("Enabling MCP with transport: {}", transport);

    Ok(())
}

#[tauri::command]
pub fn disable_mcp() -> Result<(), String> {
    log::info!("Disabling MCP");

    Ok(())
}
