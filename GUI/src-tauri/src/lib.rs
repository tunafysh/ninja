#![allow(unused_variables)]
mod db;

use db::{Database, Shuriken};
use std::{process::{Command, Stdio}, sync::Arc};
use tokio::sync::Mutex;
use tauri::{Manager, State};

struct AppState {
    db: Arc<Mutex<Database>>,
}

#[tauri::command]
async fn create_shuriken(
    state: State<'_, AppState>,
    id: String,
    name: String,
    binary_path: String,
    config_path: String,
) -> Result<(), String> {
    let shuriken = Shuriken {
        id,
        name,
        binary_path,
        config_path,
        status: "stopped".to_string(),
        pid: None,
    };
    state.db.lock().await.create_shuriken(shuriken).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_shuriken(state: State<'_, AppState>, id: String) -> Result<Option<Shuriken>, String> {
    state.db.lock().await.get_shuriken(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_all_shurikens(state: State<'_, AppState>) -> Result<Vec<Shuriken>, String> {
    state.db.lock().await.get_all_shurikens().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_shuriken_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
    pid: Option<i32>,
) -> Result<(), String> {
    state.db.lock().await.update_shuriken_status(&id, &status, pid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_shuriken(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.db.lock().await.delete_shuriken(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_shuriken(
    state: State<'_, AppState>,
    id: String,
) -> Result<Shuriken, String> {
    let db = state.db.lock().await;
    let mut shuriken = db.get_shuriken(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Shuriken not found".to_string())?;

    if shuriken.status == "running" {
        return Ok(shuriken);
    }

    // Start the process
    let process = Command::new(&shuriken.binary_path)
        .args(["-f", &shuriken.config_path]) // Example args
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start: {}", e))?;

    // Update database record
    shuriken.pid = Some(process.id() as i32);
    shuriken.status = "running".to_string();

    db.update_shuriken(shuriken.clone()).await.map_err(|e| e.to_string())?;
    Ok(shuriken)
}
#[tauri::command]
async fn stop_shuriken(
    state: State<'_, AppState>,
    id: String,
) -> Result<Shuriken, String> {
    let db = state.db.lock().await;
    let mut shuriken = db.get_shuriken(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Shuriken not found".to_string())?;

    if shuriken.status == "stopped" {
        return Ok(shuriken);
    }

    // Kill the process based on PID
    if let Some(pid) = shuriken.pid {
        // Special case for Apache
        if shuriken.id == "apache" {
            Command::new("apachectl")
                .args(["-k", "stop"])
                .status()
                .map_err(|e| format!("Failed to stop Apache: {}", e))?;
        } else {
            // Use platform-specific kill commands
            #[cfg(target_os = "windows")]
            {
                Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .status()
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
            }
            
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .status()
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
            }
        }
    }

    // Update database record
    shuriken.pid = None;
    shuriken.status = "stopped".to_string();
    
    db.update_shuriken(shuriken.clone()).await.map_err(|e| e.to_string())?;
    Ok(shuriken)
}

#[tauri::command]
async fn restart_shuriken(
    state: State<'_, AppState>,
    id: String,
) -> Result<Shuriken, String> {
    // First stop the shuriken
    let _ = stop_shuriken(state.clone(), id.clone()).await?;
    
    // Then start it again
    start_shuriken(state, id).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .filter(|metadata| metadata.target() != "tao")
            .build())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db: Arc::new(Mutex::new(
                tokio::runtime::Runtime::new()
                    .expect("Failed to create runtime")
                    .block_on(async {
                        Database::new().await.expect("Failed to initialize database")
                    })
            ))
        })
        .invoke_handler(tauri::generate_handler![
            create_shuriken,
            get_shuriken,
            get_all_shurikens,
            update_shuriken_status,
            delete_shuriken,
            start_shuriken,
            stop_shuriken,
            restart_shuriken
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            window.set_shadow(true).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
