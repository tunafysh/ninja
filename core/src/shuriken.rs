use crate::manager::ShurikenManager;
use crate::{common::types::FieldValue, scripting::NinjaEngine, scripting::templater::Templater};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tool {
    pub name: String,
    pub script: PathBuf,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenConfig {
    #[serde(rename = "config-path")]
    pub config_path: PathBuf,
    pub options: Option<HashMap<String, FieldValue>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenMetadata {
    pub name: String,
    pub id: String,
    pub version: String,
    #[serde(rename = "script-path")]
    pub script_path: PathBuf,
    #[serde(rename = "type")]
    pub shuriken_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogsConfig {
    #[serde(rename = "log-path")]
    pub log_path: PathBuf,
}

async fn atomic_write_json(path: &Path, value: &JsonValue) -> Result<(), String> {
    use tokio::fs;

    let tmp_path = path.with_extension("tmp");

    let data = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, data)
        .await
        .map_err(|e| format!("Failed to write tmp lockfile: {e}"))?;

    // Atomic-ish replace on most platforms
    std::fs::rename(&tmp_path, path).map_err(|e| format!("Failed to replace lockfile: {e}"))?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shuriken {
    #[serde(rename = "shuriken")]
    pub metadata: ShurikenMetadata,
    pub config: Option<ShurikenConfig>,
    pub logs: Option<LogsConfig>,
    pub tools: Option<Vec<Tool>>,
}

impl Shuriken {
    pub async fn start(
        &self,
        engine: &NinjaEngine,
        shuriken_dir: &Path,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), String> {
        info!("Starting shuriken {}", self.metadata.name);

        let lock_dir = shuriken_dir.join(".ninja");
        tokio::fs::create_dir_all(&lock_dir)
            .await
            .map_err(|e| format!("Failed to create .ninja directory: {}", e))?;
        let lock_path = lock_dir.join("shuriken.lck");

        let full_script_path = self.resolve_script_path(&self.metadata.script_path, shuriken_dir);

        let stem = full_script_path
            .file_stem()
            .ok_or_else(|| "Invalid script path".to_string())?
            .to_string_lossy()
            .to_string();
        let compiled_path = lock_dir.join(format!("{stem}.ns"));

        if let Some(mgr) = mgr {
            engine
                .execute_function("start", &compiled_path, Some(shuriken_dir), Some(mgr))
                .map_err(|e| format!("Script start failed: {}", e))?;
        }

        let lockfile_data = json!({
            "name": self.metadata.name,
            "type": "Script",
        });

        atomic_write_json(&lock_path, &lockfile_data).await?;

        Ok(())
    }

    pub async fn lockpick(&self, root_path: &Path) -> anyhow::Result<()> {
        let root_path = root_path
            .join("shurikens")
            .join(&self.metadata.name)
            .join(".ninja");

        if root_path.join("shuriken.lck").exists() {
            fs::remove_file(root_path.join("shuriken.lck")).await?;
        }

        Ok(())
    }

    pub async fn configure(&self, root_path: &Path) -> anyhow::Result<()> {
        if let Some(ctx) = &self.config {
            let shuriken_fields = ctx.options.clone();
            let mut fields = HashMap::new();
            if let Some(partial_fields) = shuriken_fields {
                for (name, value) in partial_fields {
                    fields.insert(name, value);
                }
            }

            // Construct full path to the shuriken folder
            let shuriken_path = root_path.join("shurikens").join(&self.metadata.name);

            // Ensure the directory exists
            fs::create_dir_all(&shuriken_path).await?;

            // Initialize Templater with the fields and shuriken path
            let templater = Templater::new(fields, shuriken_path.clone())?;

            // Full path to write the generated config
            let config_full_path = shuriken_path.join(&ctx.config_path);

            // Ensure the parent directory of the config file exists
            if let Some(parent) = config_full_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            templater
                .generate_config(config_full_path)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn import(
        &self,
        engine: &NinjaEngine,
        shuriken_dir: &Path,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), String> {
        info!("Shuriken {} is importing data!", &self.metadata.name);

        let lock_dir = shuriken_dir.join(".ninja");
        tokio::fs::create_dir_all(&lock_dir)
            .await
            .map_err(|e| format!("Failed to create .ninja directory: {}", e))?;

        let full_script_path = self.resolve_script_path(&self.metadata.script_path, shuriken_dir);

        let stem = full_script_path
            .file_stem()
            .ok_or_else(|| "Invalid script".to_string())?
            .to_string_lossy()
            .to_string();
        let lock_dir = shuriken_dir.join(".ninja");
        let compiled_path = lock_dir.join(format!("{stem}.ns"));

        engine
            .execute_function("import", &compiled_path, Some(shuriken_dir), mgr)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn resolve_script_path(&self, script_path: &Path, shuriken_dir: &Path) -> PathBuf {
        if script_path.is_absolute() {
            script_path.to_path_buf()
        } else {
            shuriken_dir.join(script_path)
        }
    }

    pub async fn stop(
        &self,
        engine: &NinjaEngine,
        shuriken_dir: &Path,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), String> {
        info!("Stopping shuriken {}", self.metadata.name);
        let lock_path = shuriken_dir.join(".ninja").join("shuriken.lck");

        let full_script_path = self.resolve_script_path(&self.metadata.script_path, shuriken_dir);

        let stem = full_script_path
            .file_stem()
            .ok_or_else(|| "Invalid script".to_string())?
            .to_string_lossy()
            .to_string();
        let lock_dir = shuriken_dir.join(".ninja");
        let compiled_path = lock_dir.join(format!("{stem}.ns"));

        if let Some(mgr) = mgr {
            engine
                .execute_function("stop", &compiled_path, Some(shuriken_dir), Some(mgr))
                .map_err(|e| format!("Script stop failed: {}", e))?;
        }

        if lock_path.exists() {
            tokio::fs::remove_file(&lock_path)
                .await
                .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
        }

        Ok(())
    }
}
