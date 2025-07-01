// service_manager.rs
#![allow(dead_code)]
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::process::Child;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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
    #[serde(rename = "bin-path")]
    pub bin_path: BinPath,
    #[serde(rename = "config-path")]
    pub config_path: String,
    pub args: Option<Vec<String>>,
    pub ports: Option<Vec<u16>>,
    #[serde(rename = "health-check")]
    pub health_check: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum BinPath {
    Simple(String),
    Platform {
        windows: String,
        unix: String,
    },
}

impl BinPath {
    pub fn get_path(&self) -> &str {
        match self {
            BinPath::Simple(path) => path,
            BinPath::Platform { windows, unix } => {
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
    pub error_log: Option<LogPath>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum LogPath {
    Simple(String),
    Platform {
        windows: String,
        unix: String,
    },
}

impl LogPath {
    pub fn get_path(&self) -> &str {
        match self {
            LogPath::Simple(path) => path,
            LogPath::Platform { windows, unix } => {
                if cfg!(windows) {
                    windows
                } else {
                    unix
                }
            }
        }
    }
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
    // Bootstrap function - initializes everything
    pub fn bootstrap() -> Result<Self, ServiceError> {
        // Initialize SQLite database
        let db = Connection::open("registry.db")
            .map_err(ServiceError::DatabaseError)?;

        // Create services table if it doesn't exist
        db.execute(
            "CREATE TABLE IF NOT EXISTS services (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                pid INTEGER
            )",
            [],
        ).map_err(ServiceError::DatabaseError)?;

        // Load service configs from TOML files
        let configs = Self::load_service_configs()?;

        Ok(Self {
            running_processes: HashMap::new(),
            service_configs: configs,
            db,
        })
    }

    // Start a service by name (not service-name)
    pub async fn start_service(&mut self, service_name: &str) -> Result<u32, ServiceError> {
        // Find the service config by name and clone it to avoid borrow conflicts
        let config = self.find_service_by_name(service_name)?.clone();
        let directory_name = self.find_directory_by_name(service_name)?;
        
        let pid = self.spawn_process(&directory_name, &config).await?;
        println!("Starting shuriken {}...", service_name);

        // Update status in database using the actual name
        self.db.execute(
            "INSERT OR REPLACE INTO services (name, status, pid) VALUES (?1, ?2, ?3)",
            [service_name, "running", &pid.to_string()],
        ).map_err(ServiceError::DatabaseError)?;

        Ok(pid)
    }

    // Stop a service by name (not service-name)
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        // First check if we have it in our running processes
        let directory_name = self.find_directory_by_name(service_name)?;
        
        if let Some(mut child) = self.running_processes.remove(&directory_name) {
            // Try graceful shutdown first
            let _ = child.kill().await;
        }

        // Also try to kill by PID from database if we have it
        if let Ok(services) = self.get_all_services().await {
            if let Some(service) = services.iter().find(|s| s.name == service_name) {
                if let Some(pid) = service.pid {
                    Self::kill_process_by_pid(pid);
                }
            }
        }

        // Update status in database
        self.db.execute(
            "INSERT OR REPLACE INTO services (name, status, pid) VALUES (?1, ?2, NULL)",
            [service_name, "stopped"],
        ).map_err(ServiceError::DatabaseError)?;

        Ok(())
    }

    // Get all services from database
    pub async fn get_all_services(&self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        let mut stmt = self.db.prepare("SELECT name, status, pid FROM services")
            .map_err(ServiceError::DatabaseError)?;

        let service_iter = stmt.query_map([], |row| {
            Ok(RuntimeStatus {
                name: row.get(0)?,
                status: row.get(1)?,
                pid: row.get(2)?,
            })
        }).map_err(ServiceError::DatabaseError)?;

        let mut services = Vec::new();
        for service in service_iter {
            services.push(service.map_err(ServiceError::DatabaseError)?);
        }

        Ok(services)
    }

    // Get running services with process verification
    pub async fn get_running_services(&mut self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        // First, clean up any stale entries
        self.cleanup_stale_processes().await?;

        let mut stmt = self.db.prepare("SELECT name, status, pid FROM services WHERE status = 'running'")
            .map_err(ServiceError::DatabaseError)?;

        let service_iter = stmt.query_map([], |row| {
            Ok(RuntimeStatus {
                name: row.get(0)?,
                status: row.get(1)?,
                pid: row.get(2)?,
            })
        }).map_err(ServiceError::DatabaseError)?;

        let mut services = Vec::new();
        for service in service_iter {
            let service = service.map_err(ServiceError::DatabaseError)?;
            
            // Double-check that the process is actually running
            if let Some(pid) = service.pid {
                if Self::is_process_running(pid) {
                    services.push(service);
                } else {
                    // Process is dead, update database
                    let _ = self.db.execute(
                        "UPDATE services SET status = 'stopped', pid = NULL WHERE name = ?1",
                        [&service.name],
                    );
                }
            }
        }

        Ok(services)
    }

    // Clean up stale process entries
    async fn cleanup_stale_processes(&mut self) -> Result<(), ServiceError> {
        let all_services = self.get_all_services().await?;
        
        for service in all_services {
            if service.status == "running" {
                if let Some(pid) = service.pid {
                    if !Self::is_process_running(pid) {
                        // Process is dead, update database
                        self.db.execute(
                            "UPDATE services SET status = 'stopped', pid = NULL WHERE name = ?1",
                            [&service.name],
                        ).map_err(ServiceError::DatabaseError)?;
                        
                        // Remove from running processes if present
                        if let Ok(directory_name) = self.find_directory_by_name(&service.name) {
                            self.running_processes.remove(&directory_name);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    // Check if a process is running by PID
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

    // Kill a process by PID
    fn kill_process_by_pid(pid: u32) {
        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .output();
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

    async fn spawn_process(&mut self, directory_name: &str, config: &ServiceConfig) -> Result<u32, ServiceError> {
        // Get the service directory path
        let service_dir = format!("shurikens/{}", directory_name);
        let bin_path = config.shuriken.bin_path.get_path();

        let mut command = tokio::process::Command::new(&bin_path);

        command.current_dir(service_dir);
        
        // Add arguments if they exist
        if let Some(args) = &config.shuriken.args {
            command.args(args);
        }

        let child = command
            .spawn()
            .map_err(|e| ServiceError::SpawnFailed(config.shuriken.name.clone(), e))?;

        let pid = child.id().ok_or(ServiceError::NoPid)?;

        self.running_processes.insert(directory_name.to_string(), child);

        Ok(pid)
    }

    fn load_service_configs() -> Result<HashMap<String, ServiceConfig>, ServiceError> {
        let mut configs = HashMap::new();
        let shurikens_dir = Path::new("shurikens");

        if !shurikens_dir.exists() {
            return Err(ServiceError::ShurikensDirectoryNotFound);
        }

        // Read all directories in shurikens/
        let entries = fs::read_dir(shurikens_dir)
            .map_err(ServiceError::IoError)?;

        for entry in entries {
            let entry = entry.map_err(ServiceError::IoError)?;
            let path = entry.path();

            if path.is_dir() {
                let directory_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(ServiceError::InvalidServiceName)?
                    .to_string();

                let manifest_path = path.join("manifest.toml");

                if manifest_path.exists() {
                    let toml_content = fs::read_to_string(&manifest_path)
                        .map_err(|e| ServiceError::ConfigReadError(directory_name.clone(), e))?;

                    let config: ServiceConfig = toml::from_str(&toml_content)
                        .map_err(|e| ServiceError::ConfigParseError(directory_name.clone(), e))?;

                    // Store by directory name, but the config contains the actual name
                    configs.insert(directory_name, config);
                }
            }
        }

        if configs.is_empty() {
            return Err(ServiceError::NoServicesFound);
        }

        Ok(configs)
    }

    // Helper to find service config by name (not service-name)
    fn find_service_by_name(&self, name: &str) -> Result<&ServiceConfig, ServiceError> {
        for config in self.service_configs.values() {
            if config.shuriken.name == name {
                return Ok(config);
            }
        }
        Err(ServiceError::ServiceNotFound(name.to_string()))
    }

    // Helper to find directory name by service name
    fn find_directory_by_name(&self, name: &str) -> Result<String, ServiceError> {
        for (directory, config) in &self.service_configs {
            if config.shuriken.name == name {
                return Ok(directory.clone());
            }
        }
        Err(ServiceError::ServiceNotFound(name.to_string()))
    }

    // Helper method to get a service config by directory name
    pub fn get_service_config(&self, directory_name: &str) -> Option<&ServiceConfig> {
        self.service_configs.get(directory_name)
    }

    // Helper method to list all available services (by name, not directory)
    pub fn list_services(&self) -> Vec<String> {
        self.service_configs.values()
            .map(|config| config.shuriken.name.clone())
            .collect()
    }
}

#[derive(Debug)]
pub enum ServiceError {
    ServiceNotFound(String),
    ProcessNotFound(String),
    SpawnFailed(String, std::io::Error),
    NoPid,
    DatabaseError(rusqlite::Error),
    ShurikensDirectoryNotFound,
    IoError(std::io::Error),
    InvalidServiceName,
    ConfigReadError(String, std::io::Error),
    ConfigParseError(String, toml::de::Error),
    NoServicesFound,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::ServiceNotFound(name) => write!(f, "Service '{}' not found", name),
            ServiceError::ProcessNotFound(name) => write!(f, "Process for service '{}' not found", name),
            ServiceError::SpawnFailed(name, err) => write!(f, "Failed to spawn service '{}': {}", name, err),
            ServiceError::NoPid => write!(f, "Could not get process ID"),
            ServiceError::DatabaseError(err) => write!(f, "Database error: {}", err),
            ServiceError::ShurikensDirectoryNotFound => write!(f, "Shurikens directory not found"),
            ServiceError::IoError(err) => write!(f, "IO error: {}", err),
            ServiceError::InvalidServiceName => write!(f, "Invalid service name"),
            ServiceError::ConfigReadError(name, err) => write!(f, "Failed to read config for '{}': {}", name, err),
            ServiceError::ConfigParseError(name, err) => write!(f, "Failed to parse config for '{}': {}", name, err),
            ServiceError::NoServicesFound => write!(f, "No services found in shurikens directory"),
        }
    }
}

impl std::error::Error for ServiceError {}