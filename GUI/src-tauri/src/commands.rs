// commands.rs
use std::collections::HashMap;

use log::{error, info};
use tokio::sync::Mutex;

use ninja::{
    dsl::{execute_commands, DslContext},
    manager::ShurikenManager,
    shuriken::{LogsConfig, ShurikenConfig, ShurikenMetadata},
    types::{FieldValue},
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Represents the actual runtime state of a shuriken
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ShurikenRuntimeState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// Unified shuriken info sent to the frontend
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenFull {
    pub metadata: ShurikenMetadata,
    pub config: Option<ShurikenConfig>,
    pub logs: Option<LogsConfig>,
    pub runtime: ShurikenRuntimeState,
}

impl ShurikenManager {
    /// Utility to set the runtime state for a shuriken
    pub async fn set_runtime_state(&self, name: &str, state: ShurikenRuntimeState) {
        let mut states = self.states.write().await;
        states.insert(name.to_string(), state);
    }

    /// Get the full shuriken info including runtime state
    pub async fn get_full(&self, name: &str) -> Result<ShurikenFull, String> {
        let s = self.get(name.to_string()).await.map_err(|e| e.to_string())?;
        let runtime = self
            .states
            .read()
            .await
            .get(&s.shuriken.name)
            .cloned()
            .unwrap_or(ShurikenRuntimeState::Stopped);

        Ok(ShurikenFull {
            metadata: s.shuriken,
            config: s.config,
            logs: s.logs,
            runtime,
        })
    }
}

#[tauri::command]
pub async fn start_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<ShurikenFull, String> {
    let manager = manager.lock().await;

    manager.set_runtime_state(name, ShurikenRuntimeState::Starting).await;

    match manager.start(name).await {
        Ok(_) => manager.set_runtime_state(name, ShurikenRuntimeState::Running).await,
        Err(e) => {
            manager.set_runtime_state(name, ShurikenRuntimeState::Stopped).await;
            return Err(format!("Failed to start shuriken: {}", e));
        }
    }

    manager.get_full(name).await
}

#[tauri::command]
pub async fn stop_shuriken(
    name: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<ShurikenFull, String> {
    let manager = manager.lock().await;

    manager.set_runtime_state(name, ShurikenRuntimeState::Stopping).await;

    match manager.stop(name).await {
        Ok(_) => manager.set_runtime_state(name, ShurikenRuntimeState::Stopped).await,
        Err(e) => {
            manager.set_runtime_state(name, ShurikenRuntimeState::Running).await;
            return Err(format!("Failed to stop shuriken: {}", e));
        }
    }

    manager.get_full(name).await
}

#[tauri::command]
pub async fn refresh_shurikens(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<ShurikenFull>, String> {
    let manager = manager.lock().await;
    manager.refresh().await.map_err(|e| e.to_string())?;

    let list = manager
        .list(false)
        .await
        .map_err(|e| e.to_string())?
        .right()
        .ok_or_else(|| "No shurikens found".to_string())?;

    let mut output = Vec::new();
    for name in list {
        if let Ok(sh) = manager.get_full(&name).await {
            output.push(sh);
        }
    }

    Ok(output)
}

#[tauri::command]
pub async fn get_running_shurikens(
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<Vec<ShurikenFull>, String> {
    let manager = manager.lock().await;
    let list = manager
        .list(true)
        .await
        .map_err(|e| e.to_string())?
        .left()
        .ok_or_else(|| "No running shurikens".to_string())?;

    let mut output = Vec::new();
    for (name, status) in list {
        if status == ShurikenRuntimeState::Running {
            if let Ok(sh) = manager.get_full(&name).await {
                output.push(sh);
            }
        }
    }

    Ok(output)
}

#[tauri::command]
pub async fn execute_dsl(
    command: &str,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<String, String> {
    let manager = manager.blocking_lock();
    let context = DslContext::new(manager.clone());
    let res = execute_commands(&context, command.to_string())
        .await
        .map_err(|e| e.to_string())?;
    Ok(res.join("\n"))
}

#[tauri::command]
pub async fn configure_shuriken(
    name: &str,
    fields: HashMap<String, FieldValue>,
    manager: State<'_, Mutex<ShurikenManager>>,
) -> Result<ShurikenFull, String> {
    let manager = manager.lock().await;
    manager.configure(name).await.map_err(|e| e.to_string())?;

    manager.get_full(name).await
}
