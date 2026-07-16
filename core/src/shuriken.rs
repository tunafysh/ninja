use crate::common::types::ShurikenState;
use crate::manager::ShurikenManager;
use crate::utils::{get_port_owner, normalize_path, parse_path};
use crate::{common::types::FieldValue, scripting::NinjaEngine, scripting::templater::Templater};
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::sync::Arc;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{fs, sync::Mutex};

/// Represents a tool script associated with a Shuriken.
///
/// Tools are executable scripts that can be invoked to perform
/// specific tasks related to the Shuriken.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// Path to the tool's script file
    pub script: PathBuf,
    /// Optional human-readable description
    pub description: Option<String>,
}

/// Configuration settings for a Shuriken.
///
/// Specifies the path to configuration templates and runtime options.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenConfig {
    /// Path to the configuration file template (relative to Shuriken directory)
    #[serde(rename = "config-path")]
    pub config_path: PathBuf,
    /// Runtime configuration options applied during execution
    pub options: Option<HashMap<String, FieldValue>>,
}

/// Metadata describing a Shuriken in shuriken.toml.
///
/// Contains identifying information, type, version, and optional startup details.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenMetadata {
    /// Human-readable name of the Shuriken
    pub name: String,
    /// Unique identifier for the Shuriken
    pub id: String,
    /// Version string (e.g., "1.0.0")
    pub version: String,
    /// Optional list of TCP ports used by the service
    pub ports: Option<Vec<u16>>,
    /// Whether to verify ports are free before starting (default: false)
    #[serde(rename = "check-ports")]
    pub check_ports: Option<bool>,
    /// Path to the main startup script
    #[serde(rename = "script-path")]
    pub script_path: Option<PathBuf>,
    /// Type of Shuriken: "daemon", "binary", "library", etc.
    #[serde(rename = "type")]
    pub shuriken_type: String,
}

/// Logging configuration for a Shuriken.
///
/// Specifies where the Shuriken should write its log files.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogsConfig {
    /// Path where logs should be written
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

/// Represents the complete TOML structure of a shuriken.toml file.
///
/// This is the raw representation before being parsed into a `Shuriken` struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenToml {
    /// Required: Shuriken metadata section
    pub shuriken: ShurikenMetadata,
    /// Optional: Configuration section
    pub config: Option<ShurikenConfig>,
    /// Optional: Logging configuration
    pub logs: Option<LogsConfig>,
    /// Optional: Available tools
    pub tools: Option<Vec<Tool>>,
}

/// A loaded and managed Shuriken service instance.
///
/// Represents an installed Shuriken with metadata, configuration, logging, and runtime state.
/// The `state` and `dirty` flags are not serialized.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shuriken {
    /// Shuriken metadata
    #[serde(rename = "shuriken")]
    pub metadata: ShurikenMetadata,
    /// Configuration settings
    pub config: Option<ShurikenConfig>,
    /// Logging configuration
    pub logs: Option<LogsConfig>,
    /// Available tools
    pub tools: Option<Vec<Tool>>,

    /// Current runtime state (Running, Idle, or Error)
    #[serde(skip)]
    pub state: Arc<Mutex<ShurikenState>>,
    /// Whether in-memory state differs from disk
    #[serde(skip)]
    pub dirty: Arc<Mutex<bool>>,
}

impl Shuriken {
    /// Starts this Shuriken by executing its startup script.
    ///
    /// Performs port availability checks if configured, creates necessary directories,
    /// compiles and executes the startup script, and writes a lock file.
    /// Updates internal state to `Running` on success.
    ///
    /// # Arguments
    /// - `engine`: Reference to the Lua scripting engine
    /// - `shuriken_dir`: Directory containing the Shuriken's files
    /// - `mgr`: Optional manager reference for script context
    ///
    /// # Returns
    /// - `Ok(())` if startup succeeded
    /// - `Err(msg)` if port check fails, script execution fails, or I/O fails
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

        if self.metadata.check_ports.unwrap_or(false) {
            info!("Checking ports for shuriken {}", self.metadata.name);
            if let Some(ports) = &self.metadata.ports {
                for port in ports {
                    let port_owner = get_port_owner(*port);
                    if let Some(owner) = port_owner {
                        let name = owner.name.unwrap_or_else(|| "Unknown".into());
                        return Err(format!(
                            "Port {} is already in use by process {}. Please free the port and try again.",
                            port, name
                        ));
                    } else {
                        info!("Port {} is free", port);
                    }
                }
            }
        }

        if self.metadata.shuriken_type == "daemon"
            && let Some(script_path) = &self.metadata.script_path
        {
            let path = normalize_path(&script_path.as_path());
            let full_script_path = parse_path(
                &shuriken_dir.to_path_buf(),
                path.display().to_string(),
                None,
            );

            let stem = full_script_path
                .file_stem()
                .ok_or_else(|| "Invalid script path".to_string())?
                .to_string_lossy()
                .to_string();
            let compiled_path = lock_dir.join(format!("{stem}.ns"));

            if let Some(mgr) = mgr {
                engine
                    .execute_function("start", &compiled_path, Some(shuriken_dir), Some(mgr))
                    .await
                    .map_err(|e| format!("Script start failed: {}", e))?;
            }

            let lockfile_data = json!({
                "name": self.metadata.name,
                "type": "Script",
            });

            atomic_write_json(&lock_path, &lockfile_data).await?;

            let mut state = self.state.lock().await;
            *state = ShurikenState::Running;
        }
        Ok(())
    }

    /// Removes the lock file for this Shuriken (without stopping it).
    ///
    /// Useful for recovering from crashes where the lock file wasn't cleaned up.
    ///
    /// # Arguments
    /// - `root_path`: The root Ninja directory
    ///
    /// # Returns
    /// - `Ok(())` if lock file removed or didn't exist
    /// - `Err` if operation fails
    pub async fn lockpick(&self, root_path: &Path) -> anyhow::Result<()> {
        let root_path = root_path
            .join("shurikens")
            .join(&self.metadata.name.to_lowercase())
            .join(".ninja");

        if root_path.join("shuriken.lck").exists() {
            fs::remove_file(root_path.join("shuriken.lck")).await?;
        }

        Ok(())
    }

    /// Configures this Shuriken by templating its configuration file.
    ///
    /// Uses the `Templater` to render configuration templates with provided field values,
    /// then executes the `post_config` function if a script path is defined.
    ///
    /// # Arguments
    /// - `root_path`: The root Ninja directory
    /// - `engine`: Reference to the Lua scripting engine
    /// - `mgr`: Optional manager reference for script context
    ///
    /// # Returns
    /// - `Ok(())` if configuration succeeded
    /// - `Err` if no config/script path exists or template/script execution fails
    pub async fn configure(
        &self,
        root_path: &Path,
        engine: &NinjaEngine,
        mgr: Option<ShurikenManager>,
    ) -> anyhow::Result<()> {
        info!("Configuring shuriken '{}'", self.metadata.name);

        if let Some(ctx) = &self.config {
            debug!("Configuration found");

            let fields = ctx
                .options
                .clone()
                .into_iter()
                .flatten()
                .collect::<HashMap<_, _>>();

            let shuriken_path = root_path
                .join("shurikens")
                .join(self.metadata.name.to_lowercase());

            debug!("Creating directory '{}'", shuriken_path.display());
            fs::create_dir_all(&shuriken_path).await?;

            let templater = Templater::new(fields, shuriken_path.clone())?;
            debug!("Templater initialized");

            let config_full_path = shuriken_path.join(&ctx.config_path);
            info!("Generating config '{}'", config_full_path.display());

            if let Some(parent) = config_full_path.parent() {
                debug!("Ensuring parent directory '{}'", parent.display());
                fs::create_dir_all(parent).await?;
            }

            match templater.generate_config(config_full_path.clone()).await {
                Ok(_) => {
                    info!("Successfully generated '{}'", config_full_path.display());
                }
                Err(e) => {
                    error!("Failed to generate config: {e}");
                    return Err(anyhow::Error::msg(e.to_string()));
                }
            }
        } else {
            warn!("Shuriken '{}' has no configuration", self.metadata.name);
        }

        if let Some(script_path) = &self.metadata.script_path
            && engine
                .check_function_exists("post_config", script_path)
                .await?
        {
            info!("Running post_config from '{}'", script_path.display());

            engine
                .execute_function("post_config", script_path, Some(root_path), mgr)
                .await?;

            info!("post_config completed");
        } else {
            debug!("No post_config script");
        }

        info!("Finished configuring '{}'", self.metadata.name);
        Ok(())
    }

    /// Imports data for this Shuriken by executing its import script.
    ///
    /// Executes the `import` function if a script path is defined.
    /// Used to prepare or initialize data before the Shuriken starts.
    ///
    /// # Arguments
    /// - `engine`: Reference to the Lua scripting engine
    /// - `shuriken_dir`: Directory containing the Shuriken's files
    /// - `mgr`: Optional manager reference for script context
    ///
    /// # Returns
    /// - `Ok(())` if import succeeded
    /// - `Err(msg)` if no script path exists or script execution fails
    pub async fn import(
        &self,
        engine: &NinjaEngine,
        shuriken_dir: &Path,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), String> {
        info!(
            "Shuriken {} is importing data! (if there is any)",
            &self.metadata.name
        );

        if let Some(script_path) = &self.metadata.script_path {
            let full_script_path = self.resolve_script_path(script_path, shuriken_dir);

            let stem = full_script_path
                .file_stem()
                .ok_or_else(|| "Invalid script".to_string())?
                .to_string_lossy()
                .to_string();
            let lock_dir = shuriken_dir.join(".ninja");
            let compiled_path = lock_dir.join(format!("{stem}.ns"));

            if let Some(mgr) = mgr {
                engine
                    .execute_function("import", &compiled_path, Some(shuriken_dir), Some(mgr))
                    .await
                    .map_err(|e| format!("Script import failed: {}", e))?;
            }
            Ok(())
        } else {
            Err("Shuriken does not have a script path".to_string())
        }
    }

    /// Resolves a script path, handling both absolute and relative paths.
    ///
    /// If the path is absolute, returns it as-is.
    /// Otherwise, joins it with the shuriken directory.
    ///
    /// # Arguments
    /// - `script_path`: The script path to resolve
    /// - `shuriken_dir`: The Shuriken's directory for relative resolution
    ///
    /// # Returns
    /// The resolved absolute path
    fn resolve_script_path(&self, script_path: &Path, shuriken_dir: &Path) -> PathBuf {
        if script_path.is_absolute() {
            script_path.to_path_buf()
        } else {
            shuriken_dir.join(script_path)
        }
    }

    /// Stops this running Shuriken wby executing its stop script.
    ///
    /// Calls the `stop` function if defined, removes the lock file,
    /// and updates internal state to `Idle`.
    ///
    /// # Arguments
    /// - `engine`: Reference to the Lua scripting engine
    /// - `shuriken_dir`: Directory containing the Shuriken's files
    /// - `mgr`: Optional manager reference for script context
    ///
    /// # Returns
    /// - `Ok(())` if stop succeeded
    /// - `Err(msg)` if script execution fails or lock file removal fails
    pub async fn stop(
        &mut self,
        engine: &NinjaEngine,
        shuriken_dir: &Path,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), String> {
        info!("Stopping shuriken {}", self.metadata.name);
        let lock_path = shuriken_dir.join(".ninja").join("shuriken.lck");

        if self.metadata.shuriken_type == "daemon"
            && let Some(script_path) = &self.metadata.script_path
        {
            let path = normalize_path(&script_path.as_path());
            let full_script_path = parse_path(
                &shuriken_dir.to_path_buf(),
                path.display().to_string(),
                None,
            );

            let stem = full_script_path
                .file_stem()
                .ok_or_else(|| "Invalid script".to_string())?
                .to_string_lossy()
                .to_string();
            let lock_dir = shuriken_dir.join(".ninja");
            let compiled_path = lock_dir.join(format!("{stem}.ns"));

            if let Some(mgr) = mgr {
                {
                    let mut state = self.state.lock().await;
                    engine
                        .execute_function("stop", &compiled_path, Some(shuriken_dir), Some(mgr))
                        .await
                        .map_err(|e| {
                            *state = ShurikenState::Error(e.to_string());
                            format!("Script stop failed: {}", e)
                        })?;
                } // Lock released here
            }

            if lock_path.exists() {
                tokio::fs::remove_file(&lock_path)
                    .await
                    .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
            }

            let mut state = self.state.lock().await;
            *state = ShurikenState::Idle;
            Ok(())
        } else {
            return Err("Shuriken does not have a script path or is not a daemon".to_string());
        }
    }
}
