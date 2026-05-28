use log::{debug, error, info};
use ninja::backup::{create_backup, restore_backup, CompressionType};
use ninja::common::config::NinjaConfig;
use ninja::common::registry::ArmoryItem;
use ninja::shuriken::{LogsConfig, Shuriken, ShurikenConfig, ShurikenMetadata, Tool};
use serde::{Serialize, Deserialize};
use std::{collections::HashMap, io::Read, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::{fs, sync::Mutex};

use ninja::{
    common::types::{ArmoryMetadata, FieldValue, ShurikenState},
    manager::ShurikenManager,
    scripting::dsl::{execute_commands, DslContext},
};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DopedShuriken {
    metadata: ShurikenMetadata,
    config: Option<ShurikenConfig>,
    logs: Option<LogsConfig>,
    tools: Option<Vec<Tool>>,
    state: ShurikenState,
}

pub trait IntoDoped {
    async fn into_doped(self) -> DopedShuriken;
}

impl IntoDoped for Shuriken {
    async fn into_doped(self) -> DopedShuriken {
        DopedShuriken {
            metadata: self.metadata,
            config: self.config,
            logs: self.logs,
            tools: self.tools,
            state: self.state.lock().await.clone(),
        }
    }
}

#[tauri::command]
pub async fn start_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    info!("Starting shuriken: {}", name);
    let manager = manager.lock().await;
    match manager.start(name).await {
        Ok(_) => info!("Shuriken {} started successfully.", name),
        Err(e) => error!("Failed to start shuriken {}: {}", name, e),
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    info!("Stopping shuriken: {}", name);
    let manager = manager.lock().await;

    match manager.stop(name).await {
        Ok(_) => info!("Shuriken {} stopped successfully.", name),
        Err(e) => error!("Failed to stop shuriken {}: {}", name, e),
    }

    Ok(())
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
) -> Result<Vec<DopedShuriken>, String> {
    info!("Retrieving all shurikens...");
    let mut output = Vec::new();
    let manager = manager.lock().await;
    if let Some(list) = manager
        .list(false)
        .await
        .map_err(|e| e.to_string())?
        .right()
    {
        info!("Found {} shurikens: {:?}", list.len(), list);
        for name in list {
            match manager.get(name.clone()).await {
                Ok(shuriken) => {
                    debug!("Retrieved shuriken: name={}, has_metadata={:#?}", name, shuriken.metadata);
                    output.push(shuriken.into_doped().await);
                }
                Err(e) => {
                    error!("Failed to get shuriken '{}': {}", name, e);
                    return Err(format!("Failed to get shuriken '{}': {}", name, e));
                }
            }
        }

        // sort shurikens by name
        output.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        info!("Successfully retrieved {} shurikens", output.len());
        Ok(output)
    } else {
        error!("No shurikens found or an internal issue occurred.");
        Err("No shurikens found or an internal issue occurred.".to_string())
    }
}

#[tauri::command]
pub async fn get_running_shurikens(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<DopedShuriken>, String> {
    info!("Retrieving running shurikens...");
    let mut output = Vec::new();
    let manager = manager.lock().await;
    if let Some(list) = manager.list(true).await.map_err(|e| e.to_string())?.left() {
        for (name, status) in list {
            if status == ShurikenState::Running {
                let shuriken = manager.get(name).await.map_err(|e| e.to_string())?;
                output.push(DopedShuriken {
                    metadata: shuriken.metadata,
                    config: shuriken.config,
                    logs: shuriken.logs,
                    tools: shuriken.tools,
                    state: status,
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

    if &header[0..6] != b"HSRZEG" {
        return Err("Invalid shuriken file".into());
    }

    let metadata_len = u16::from_le_bytes([header[6], header[7]]);
    let mut metadata_buf = vec![0u8; metadata_len.into()];
    file.read_exact(&mut metadata_buf)
        .map_err(|e| e.to_string())?;

    let metadata: ArmoryMetadata =
        serde_cbor::from_slice(&metadata_buf).map_err(|e| e.to_string())?;

    Ok(metadata)
}

#[tauri::command]
pub async fn install_shuriken(
    manager: State<'_, Mutex<ShurikenManager>>,
    path: String,
) -> Result<(), String> {
    let manager = manager.lock().await;
    manager
        .install(&PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn remove_shuriken(
    manager: State<'_, Mutex<ShurikenManager>>,
    name: String,
) -> Result<(), String> {
    let manager = manager.lock().await;
    manager.remove(&name).await.map_err(|e| e.to_string())?;
    Ok(())
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

#[tauri::command]
pub async fn lockpick_shuriken(
    manager: State<'_, Mutex<ShurikenManager>>,
    shuriken: String,
) -> Result<(), String> {
    let manager = manager.lock().await;
    manager
        .lockpick(&shuriken)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(manager: State<'_, Mutex<ShurikenManager>>) -> Result<NinjaConfig, String> {
    let manager = manager.lock().await;
    let config = manager.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn toggle_dev_mode(manager: State<'_, Mutex<ShurikenManager>>) -> Result<(), String> {
    let manager = manager.lock().await;
    let mut config = manager.config.write().await;
    let dev_mode = !config.dev_mode;
    config.set_dev_mode(dev_mode);
    Ok(())
}

#[tauri::command]
pub async fn add_registry(
    name: String,
    url: String,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    let manager = manager.lock().await;
    let mut config = manager.config.write().await;
    config.add_registry(name, url);
    Ok(())
}

#[tauri::command]
pub async fn toggle_updates(manager: State<'_, Mutex<ShurikenManager>>) -> Result<(), String> {
    let manager = manager.lock().await;
    let mut config = manager.config.write().await;
    let value = !config.check_updates;
    config.set_check_updates(value);
    Ok(())
}

#[tauri::command]
pub async fn remove_registry(
    manager: State<'_, Mutex<ShurikenManager>>,
    name: String,
) -> Result<(), String> {
    let manager = manager.lock().await;
    let mut config = manager.config.write().await;
    config.remove_registry(&name);
    Ok(())
}

#[tauri::command]
pub async fn config_exists(manager: State<'_, Mutex<ShurikenManager>>) -> Result<bool, String> {
    let manager = manager.lock().await;
    if manager.root_path.join("config.toml").exists() {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn registry_get_all_shurikens(
    manager: tauri::State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<ArmoryItem>, String> {
    let manager = manager.lock().await;
    let result = manager.registry_get_all_shurikens().await;
    Ok(result)
}

#[tauri::command]
pub async fn registry_get_shuriken(
    manager: tauri::State<'_, Mutex<ShurikenManager>>,
    name: String,
) -> Result<ArmoryItem, String> {
    let manager = manager.lock().await;
    let result = manager
        .registry_get_shuriken(name)
        .await
        .ok_or_else(|| "Shuriken not found in registries".to_string())?;
    Ok(result)
}

#[tauri::command]
pub async fn open_devtools(
    app: AppHandle,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<(), String> {
    let manager = manager.lock().await;

    let dev_mode = manager.config.read().await.dev_mode;

    if dev_mode {
        match app.webview_windows().get("main") {
            Some(window) => {
                window.open_devtools();
            }
            None => error!("Main window not found, cannot open devtools."),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn read_logs(
    manager: State<'_, Mutex<ShurikenManager>>,
    shuriken_name: &str,
) -> Result<Vec<String>, String> {
    let manager = manager.lock().await;
    let shuriken = manager
        .get(shuriken_name.to_string())
        .await
        .map_err(|e| e.to_string())?;
    let log_path = &shuriken
        .logs
        .as_ref()
        .ok_or("Shuriken does not have logs configured")?
        .log_path;
    let full_path = manager
        .root_path
        .join("shurikens")
        .join(shuriken_name)
        .join(&log_path);

    log::info!("Reading logs from path: {}", full_path.display());

    match fs::read_to_string(&full_path).await {
        Ok(content) => {
            // Get last 50 lines
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            Ok(lines)
        }
        Err(e) => Err(format!("Failed to read logs: {}", e)),
    }
}
