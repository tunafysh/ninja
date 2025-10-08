use log::{error, info};
use tokio::sync::Mutex;

use ninja::{
    dsl::{execute_commands, DslContext},
    manager::ShurikenManager,
    shuriken::{LogsConfig, Shuriken, ShurikenConfig, ShurikenMetadata},
    types::ShurikenState,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenWithStatus {
    pub shuriken: ShurikenMetadata,
    pub config: Option<ShurikenConfig>,
    pub logs: Option<LogsConfig>,
    pub status: ShurikenState,
}

#[tauri::command]
pub async fn start_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    info!("Starting shuriken: {}", name);
    let manager = manager.lock().await;
    match manager.start(name).await {
        Ok(_) => {
            info!("Shuriken {} started successfully.", name);
            Ok(())
        }
        Err(e) => {
            error!("Failed to start shuriken {}: {}", name, e);
            Err(format!("Failed to start shuriken: {}", e))
        }
    }
}

#[tauri::command]
pub async fn stop_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    info!("Stopping shuriken: {}", name);
    let manager = manager.lock().await;
    match manager.stop(name).await {
        Ok(_) => {
            info!("Shuriken {} stopped successfully.", name);
            Ok(())
        }
        Err(e) => {
            error!("Failed to stop shuriken {}: {}", name, e);
            Err(format!("Failed to stop shuriken: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_all_shurikens(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<ShurikenWithStatus>, String> {
    info!("Retrieving all shurikens...");
    let mut output = Vec::new();
    let manager = manager.lock().await;
    if let Some(list) = manager
        .list(false)
        .await
        .map_err(|e| e.to_string())?
        .right()
    {
        for name in list {
            let shuriken = manager.get(name.clone()).await.map_err(|e| e.to_string())?;
            if let Some(partial_status) = manager.states.read().await.get(&name) {
                let status = partial_status.clone();
                output.push(ShurikenWithStatus {
                    shuriken: shuriken.shuriken,
                    config: shuriken.config,
                    logs: shuriken.logs,
                    status,
                });
            }
        }
        Ok(output)
    } else {
        error!("No shurikens found or an internal issue occurred.");
        Err("No shurikens found or an internal issue occurred.".to_string())
    }
}

#[tauri::command]
pub async fn get_running_shurikens(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<ShurikenWithStatus>, String> {
    info!("Retrieving running shurikens...");
    let mut output = Vec::new();
    let manager = manager.lock().await;
    if let Some(list) = manager.list(true).await.map_err(|e| e.to_string())?.left() {
        for (name, status) in list {
            if status == ShurikenState::Running {
                let shuriken = manager.get(name).await.map_err(|e| e.to_string())?;
                output.push(ShurikenWithStatus {
                    shuriken: shuriken.shuriken,
                    config: shuriken.config,
                    logs: shuriken.logs,
                    status,
                });
            }
        }
        Ok(output)
    } else {
        Err("No running shurikens found or an internal issue occurred.".to_string())
    }
}

#[tauri::command]
pub async fn execute_dsl(
    command: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<String, String> {
    info!("Executing command {}", command);
    let manager = manager.blocking_lock();
    let context = DslContext::new(manager.clone());
    let res = execute_commands(&context, command.to_string())
        .await
        .map_err(|e| e.to_string())?;
    Ok(res.join("\n"))
}

#[tauri::command]
pub async fn configure_shuriken(shuriken: Shuriken) -> Result<(), String> {
    shuriken.configure().await.map_err(|e| e.to_string())?;
    Ok(())
}
