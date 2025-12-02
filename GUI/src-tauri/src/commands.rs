use log::{error, info};
use ninja::backup::{create_backup, restore_backup, CompressionType};
use serde_cbor;
use std::{collections::HashMap, io::Read, path::PathBuf};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::{fs, sync::Mutex};

use ninja::{
    dsl::{execute_commands, DslContext},
    manager::{ArmoryMetadata, ShurikenManager},
    shuriken::{LogsConfig, ShurikenConfig, ShurikenMetadata},
    types::{FieldValue, ShurikenState},
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenWithStatus {
    pub metadata: ShurikenMetadata,
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
pub async fn refresh_shurikens(manager: State<'_, Mutex<ShurikenManager>>) -> Result<(), String> {
    info!("Refresh shurikens");
    let manager = manager.lock().await;
    match manager.refresh().await {
        Ok(_) => {
            info!("Shurikens refreshed successfully.");
            Ok(())
        }
        Err(e) => {
            error!("Failed to refresh shurikens: {}", e);
            Err(format!("Failed to refresh shuriken: {}", e))
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
                    metadata: shuriken.metadata,
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
                    metadata: shuriken.metadata,
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
) -> Result<Vec<String>, String> {
    info!("Executing command {}", command);
    let manager = manager.lock().await;
    let context = DslContext::new(manager.clone());

    let res = execute_commands(&context, command.to_string())
        .await
        .map_err(|e| e.to_string())?;

    Ok(res) // return Vec<String> directly
}

#[tauri::command]
pub async fn configure_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    let manager = manager.lock().await;
    manager.configure(name).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn developer_mode() -> bool {
    // true if running a debug build
    cfg!(debug_assertions)
}

#[tauri::command]
pub async fn open_dir(
    manager: State<'_, Mutex<ShurikenManager>>,
    app: AppHandle,
    path: &str,
) -> Result<(), String> {
    let manager = manager.lock().await;
    let path = manager.root_path.join(path);
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(
    manager: State<'_, Mutex<ShurikenManager>>,
    name: &str,
    data: HashMap<String, FieldValue>,
) -> Result<(), String> {
    let manager = manager.lock().await;
    manager
        .save_config(name, data)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_projects(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<String>, String> {
    let manager = manager.lock().await;
    manager.get_projects().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_readme(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<String, String> {
    let manager = manager.lock().await;
    let root = &manager.root_path;
    let project = root.join("projects").join(name);

    // Common README file variants
    let readme_files = ["README.md", "README.MD", "README", "readme.md", "readme"];

    for filename in &readme_files {
        let path: PathBuf = project.join(filename);
        if fs::metadata(&path).await.is_ok() {
            match fs::read_to_string(&path).await {
                Ok(content) => return Ok(content),
                Err(e) => return Err(format!("Failed to read {}: {}", filename, e)),
            }
        }
    }

    Ok("".to_string()) // no README found
}

#[tauri::command]
pub fn open_shuriken(path: String) -> Result<ArmoryMetadata, String> {
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 8];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;

    if &header[0..2] != b"HS" {
        return Err("Invalid shuriken file".into());
    }

    let metadata_len = u16::from_le_bytes([header[3], header[4]]);
    let mut metadata_buf = vec![0u8; metadata_len.into()];
    file.read_exact(&mut metadata_buf)
        .map_err(|e| e.to_string())?;

    let metadata: ArmoryMetadata =
        serde_cbor::from_slice(&metadata_buf).map_err(|e| e.to_string())?;

    Ok(metadata)
}

#[tauri::command]
pub async fn backup_now(
    level: CompressionType,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    let manager = manager.lock().await;
    create_backup(&manager, Some(level))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn backup_restore(
    file: String,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    let manager = manager.lock().await;
    let path = PathBuf::from(file);
    restore_backup(&manager, &path)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
