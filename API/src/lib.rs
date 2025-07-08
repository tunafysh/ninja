#![allow(dead_code)]
use ninja_engine::NinjaEngine;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Child;

// Simplified config structures using platform-aware paths
#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    pub shuriken: ShurikenConfig,
    pub config: Option<HashMap<String, ConfigParam>>,
    pub logs: Option<LogsConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShurikenConfig {
    pub name: String,
    #[serde(rename = "service-name")]
    pub service_name: String,
    pub maintenance: MaintenanceType,
    #[serde(rename = "bin-path")]
    pub bin_path: Option<PlatformPath>,
    #[serde(rename = "script-path")]
    pub script_path: Option<PathBuf>,
    #[serde(rename = "config-path")]
    pub config_path: Option<PathBuf>,
    pub args: Option<Vec<String>>,
    #[serde(flatten)]
    pub shuriken_type: ShurikenType,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MaintenanceType {
    Native {
        maintenance: String,
        #[serde(rename = "bin-path")]
        bin_path: PlatformPath,
        #[serde(rename = "config-path")]
        config_path: PathBuf,
        args: Option<Vec<String>>,
    },
    Script {
        maintenance: String,
        #[serde(rename = "script-path")]
        script_path: PathBuf,
    },
    Simple(String),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ShurikenType {
    Daemon {
        r#type: String,
        ports: Option<Vec<u16>>,
        #[serde(rename = "health-check")]
        health_check: Option<String>,
    },
    Executable {
        r#type: String,
        #[serde(rename = "add-path")]
        add_path: bool,
    },
}

// Unified platform-aware path type
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum PlatformPath {
    Simple(String),
    Platform { windows: String, unix: String },
}

impl PlatformPath {
    pub fn get_path(&self) -> &str {
        match self {
            PlatformPath::Simple(path) => path,
            PlatformPath::Platform { windows, unix } => {
                if cfg!(windows) {
                    windows
                } else {
                    unix
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigParam {
    pub input: String,
    pub default: Option<toml::Value>,
    pub script: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogsConfig {
    #[serde(rename = "error-log")]
    pub error_log: Option<PlatformPath>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
}

pub struct ServiceManager {
    running_processes: HashMap<String, Child>,
    service_configs: HashMap<String, ServiceConfig>,
    db: Connection,
}

impl ServiceManager {
    pub fn bootstrap() -> Result<Self, ServiceError> {
        let db = Connection::open("registry.db")?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS services (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                pid INTEGER
            )",
            [],
        )?;

        let configs = Self::load_service_configs()?;

        Ok(Self {
            running_processes: HashMap::new(),
            service_configs: configs,
            db,
        })
    }

    pub async fn start_service(&mut self, service_name: &str) -> Result<u32, ServiceError> {
        let config = self.find_service_by_name(service_name)?.clone();
        let directory_name = self.find_directory_by_name(service_name)?;

        let pid = self.spawn_process(&directory_name, &config).await?;
        println!("Starting shuriken {}...", service_name);

        self.update_service_status(service_name, "running", Some(pid))?;
        Ok(pid)
    }

    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        let directory_name = self.find_directory_by_name(service_name)?;

        // Stop process if we're managing it
        if let Some(mut child) = self.running_processes.remove(&directory_name) {
            let _ = child.kill().await;
        }

        // Also kill by PID from database
        if let Some(pid) = self.get_service_pid(service_name)? {
            Self::kill_process_by_pid(pid);
        }

        self.update_service_status(service_name, "stopped", None)?;
        Ok(())
    }

    pub async fn get_all_services(&self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        let mut stmt = self.db.prepare("SELECT name, status, pid FROM services")?;
        let service_iter = stmt.query_map([], |row| {
            Ok(RuntimeStatus {
                name: row.get(0)?,
                status: row.get(1)?,
                pid: row.get(2)?,
            })
        })?;

        service_iter
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn get_running_services(&mut self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        self.cleanup_stale_processes().await?;

        let mut stmt = self
            .db
            .prepare("SELECT name, status, pid FROM services WHERE status = 'running'")?;
        let service_iter = stmt.query_map([], |row| {
            Ok(RuntimeStatus {
                name: row.get(0)?,
                status: row.get(1)?,
                pid: row.get(2)?,
            })
        })?;

        let mut services = Vec::new();
        for service in service_iter {
            let service = service?;

            if let Some(pid) = service.pid {
                if Self::is_process_running(pid) {
                    services.push(service);
                } else {
                    // Process died, update status
                    self.update_service_status(&service.name, "stopped", None)?;
                }
            }
        }

        Ok(services)
    }

    pub fn list_services(&self) -> Vec<String> {
        self.service_configs
            .values()
            .map(|config| config.shuriken.name.clone())
            .collect()
    }

    pub fn get_service_config(&self, directory_name: &str) -> Option<&ServiceConfig> {
        self.service_configs.get(directory_name)
    }

    // Private helper methods
    async fn cleanup_stale_processes(&mut self) -> Result<(), ServiceError> {
        let all_services = self.get_all_services().await?;

        for service in all_services {
            if service.status == "running" {
                if let Some(pid) = service.pid {
                    if !Self::is_process_running(pid) {
                        self.update_service_status(&service.name, "stopped", None)?;

                        if let Ok(directory_name) = self.find_directory_by_name(&service.name) {
                            self.running_processes.remove(&directory_name);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn spawn_process(
        &mut self,
        directory_name: &str,
        config: &ServiceConfig,
    ) -> Result<u32, ServiceError> {
        let service_dir = format!("shurikens/{}", directory_name);

        // Extract execution details from maintenance config
        let (bin_path, args) = self.extract_execution_details(config)?;

        // Handle script execution via ninja_engine
        if bin_path.is_empty() {
            return Ok(0); // Placeholder PID for ninja_engine managed processes
        }

        let mut command = tokio::process::Command::new(&bin_path);
        command.current_dir(service_dir);

        if !args.is_empty() {
            command.args(&args);
        }

        let child = command
            .spawn()
            .map_err(|e| ServiceError::SpawnFailed(config.shuriken.name.clone(), e))?;

        let pid = child.id().ok_or(ServiceError::NoPid)?;
        self.running_processes
            .insert(directory_name.to_string(), child);

        Ok(pid)
    }

    fn extract_execution_details(
        &self,
        config: &ServiceConfig,
    ) -> Result<(String, Vec<String>), ServiceError> {
        match &config.shuriken.maintenance {
            MaintenanceType::Native { bin_path, args, .. } => Ok((
                bin_path.get_path().to_string(),
                args.clone().unwrap_or_default(),
            )),
            MaintenanceType::Script { script_path, .. } => {
                self.execute_ninja_script(script_path, &config.shuriken.name)?;
                Ok((String::new(), Vec::new()))
            }
            MaintenanceType::Simple(maintenance_type) => match maintenance_type.as_str() {
                "native" => {
                    let bin_path = config.shuriken.bin_path.as_ref().ok_or_else(|| {
                        ServiceError::ConfigError(
                            "bin-path required for native maintenance".to_string(),
                        )
                    })?;
                    Ok((
                        bin_path.get_path().to_string(),
                        config.shuriken.args.clone().unwrap_or_default(),
                    ))
                }
                "script" => {
                    let script_path = config.shuriken.script_path.as_ref().ok_or_else(|| {
                        ServiceError::ConfigError(
                            "script-path required for script maintenance".to_string(),
                        )
                    })?;
                    self.execute_ninja_script(script_path, &config.shuriken.name)?;
                    Ok((String::new(), Vec::new()))
                }
                _ => Err(ServiceError::ConfigError(
                    "maintenance must be 'native' or 'script'".to_string(),
                )),
            },
        }
    }

    fn execute_ninja_script(
        &self,
        script_path: &PathBuf,
        service_name: &str,
    ) -> Result<(), ServiceError> {
        let engine = NinjaEngine::new();
        engine
            .execute_function("start".to_string(), script_path)
            .map_err(|e| {
                ServiceError::SpawnFailed(
                    service_name.to_string(),
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Ninja engine error: {}", e),
                    ),
                )
            })?;
        Ok(())
    }

    fn load_service_configs() -> Result<HashMap<String, ServiceConfig>, ServiceError> {
        let shurikens_dir = Path::new("shurikens");

        if !shurikens_dir.exists() {
            return Err(ServiceError::ShurikensDirectoryNotFound);
        }

        let mut configs = HashMap::new();

        for entry in fs::read_dir(shurikens_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let directory_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(ServiceError::InvalidServiceName)?
                    .to_string();

                let manifest_path = path.join("manifest.toml");
                if manifest_path.exists() {
                    let toml_content = fs::read_to_string(&manifest_path)?;
                    let config: ServiceConfig = toml::from_str(&toml_content)
                        .map_err(|e| ServiceError::ConfigParseError(directory_name.clone(), e))?;

                    configs.insert(directory_name, config);
                }
            }
        }

        if configs.is_empty() {
            return Err(ServiceError::NoServicesFound);
        }

        Ok(configs)
    }

    fn find_service_by_name(&self, name: &str) -> Result<&ServiceConfig, ServiceError> {
        self.service_configs
            .values()
            .find(|config| config.shuriken.name == name)
            .ok_or_else(|| ServiceError::ServiceNotFound(name.to_string()))
    }

    fn find_directory_by_name(&self, name: &str) -> Result<String, ServiceError> {
        self.service_configs
            .iter()
            .find(|(_, config)| config.shuriken.name == name)
            .map(|(directory, _)| directory.clone())
            .ok_or_else(|| ServiceError::ServiceNotFound(name.to_string()))
    }

    fn update_service_status(
        &self,
        service_name: &str,
        status: &str,
        pid: Option<u32>,
    ) -> Result<(), ServiceError> {
        match pid {
            Some(pid) => {
                self.db.execute(
                    "INSERT OR REPLACE INTO services (name, status, pid) VALUES (?1, ?2, ?3)",
                    [service_name, status, &pid.to_string()],
                )?;
            }
            None => {
                self.db.execute(
                    "INSERT OR REPLACE INTO services (name, status, pid) VALUES (?1, ?2, NULL)",
                    [service_name, status],
                )?;
            }
        }
        Ok(())
    }

    fn get_service_pid(&self, service_name: &str) -> Result<Option<u32>, ServiceError> {
        let mut stmt = self
            .db
            .prepare("SELECT pid FROM services WHERE name = ?1")?;
        let mut rows = stmt.query_map([service_name], |row| Ok(row.get::<_, Option<u32>>(0)?))?;

        if let Some(row) = rows.next() {
            Ok(row?)
        } else {
            Ok(None)
        }
    }

    fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {}", pid))
                .output()
                .map(|output| {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    output_str.contains(&pid.to_string())
                })
                .unwrap_or(false)
        }
    }

    fn kill_process_by_pid(pid: u32) {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .arg("/PID")
                .arg(pid.to_string())
                .arg("/F")
                .output();
        }
    }
}

// Simplified error handling with automatic conversions
#[derive(Debug)]
pub enum ServiceError {
    ServiceNotFound(String),
    SpawnFailed(String, std::io::Error),
    NoPid,
    ConfigError(String),
    ShurikensDirectoryNotFound,
    InvalidServiceName,
    ConfigParseError(String, toml::de::Error),
    NoServicesFound,
    DatabaseError(rusqlite::Error),
    IoError(std::io::Error),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::ServiceNotFound(name) => write!(f, "Service '{}' not found", name),
            ServiceError::SpawnFailed(name, err) => {
                write!(f, "Failed to spawn service '{}': {}", name, err)
            }
            ServiceError::NoPid => write!(f, "Could not get process ID"),
            ServiceError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ServiceError::ShurikensDirectoryNotFound => write!(f, "Shurikens directory not found"),
            ServiceError::InvalidServiceName => write!(f, "Invalid service name"),
            ServiceError::ConfigParseError(name, err) => {
                write!(f, "Failed to parse config for '{}': {}", name, err)
            }
            ServiceError::NoServicesFound => write!(f, "No services found in shurikens directory"),
            ServiceError::DatabaseError(err) => write!(f, "Database error: {}", err),
            ServiceError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ServiceError {}

// Automatic error conversions
impl From<rusqlite::Error> for ServiceError {
    fn from(err: rusqlite::Error) -> Self {
        ServiceError::DatabaseError(err)
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}

