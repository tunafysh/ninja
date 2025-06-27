use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::process::Child;
use surrealdb::{Surreal, engine::local::{RocksDb, Db}};
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
    db: Surreal<Db>,
}

impl ServiceManager {
    // Bootstrap function - initializes everything
    pub async fn bootstrap() -> Result<Self, ServiceError> {
        // Initialize SurrealDB
        let db = Surreal::new::<RocksDb>("registry.db").await
            .map_err(ServiceError::DatabaseError)?;
        
        // Use default namespace and database
        db.use_ns("ninja").use_db("services").await
            .map_err(ServiceError::DatabaseError)?;
        
        // Load service configs from TOML files
        let configs = Self::load_service_configs()?;
        
        Ok(Self {
            running_processes: HashMap::new(),
            service_configs: configs,
            db,
        })
    }
    
    // Start a service
    pub async fn start_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        let pid = self.spawn_process(service_name).await?;
        println!("Starting shuriken {}...", service_name);
        // Update status in database
        self.db.update(("status", service_name)).content(RuntimeStatus {
            name: service_name.to_string(),
            status: "running".to_string(),
            pid: Some(pid),
        }).await.map_err(ServiceError::DatabaseError)?;
        
        Ok(())
    }
    
    // Stop a service
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        if let Some(mut child) = self.running_processes.remove(service_name) {
            // Try graceful shutdown first
            let _ = child.kill().await;
            
            // Update status in database
            self.db.update(("status", service_name)).content(RuntimeStatus {
                name: service_name.to_string(),
                status: "stopped".to_string(),
                pid: None,
            }).await.map_err(ServiceError::DatabaseError)?;
            
            Ok(())
        } else {
            Err(ServiceError::ProcessNotFound(service_name.to_string()))
        }
    }
    
    // Get running services
    pub async fn get_running_services(&self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        let services: Vec<RuntimeStatus> = self.db
            .query("SELECT * FROM status WHERE status = 'running'")
            .await
            .map_err(ServiceError::DatabaseError)?
            .take(0)
            .map_err(ServiceError::DatabaseError)?;
        
        Ok(services)
    }
    
    async fn spawn_process(&mut self, service_name: &str) -> Result<u32, ServiceError> {
        let config = self.service_configs.get(service_name)
            .ok_or_else(|| ServiceError::ServiceNotFound(service_name.to_string()))?;
        
        // Get the service directory path
        let service_dir = format!("shurikens/{}", service_name);
        let bin_path = config.shuriken.bin_path.get_path();
        let full_bin_path = Path::new(&service_dir).join(bin_path);
        
        let mut cmd = if cfg!(windows) {
            let c = tokio::process::Command::new(&full_bin_path);
            c
        } else {
            tokio::process::Command::new(&full_bin_path)
        };
        
        cmd.current_dir(&service_dir)
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());
        
        let child = cmd.spawn()
            .map_err(|e| ServiceError::SpawnFailed(service_name.to_string(), e))?;
        
        let pid = child.id().ok_or(ServiceError::NoPid)?;
        
        self.running_processes.insert(service_name.to_string(), child);
        
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
                let service_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(ServiceError::InvalidServiceName)?
                    .to_string();
                
                let manifest_path = path.join("manifest.toml");
                
                if manifest_path.exists() {
                    let toml_content = fs::read_to_string(&manifest_path)
                        .map_err(|e| ServiceError::ConfigReadError(service_name.clone(), e))?;
                    
                    let config: ServiceConfig = toml::from_str(&toml_content)
                        .map_err(|e| ServiceError::ConfigParseError(service_name.clone(), e))?;
                    
                    configs.insert(service_name, config);
                }
            }
        }
        
        if configs.is_empty() {
            return Err(ServiceError::NoServicesFound);
        }
        
        Ok(configs)
    }
    
    // Helper method to get a service config
    pub fn get_service_config(&self, service_name: &str) -> Option<&ServiceConfig> {
        self.service_configs.get(service_name)
    }
    
    // Helper method to list all available services
    pub fn list_services(&self) -> Vec<&String> {
        self.service_configs.keys().collect()
    }
}

#[derive(Debug)]
pub enum ServiceError {
    ServiceNotFound(String),
    ProcessNotFound(String),
    SpawnFailed(String, std::io::Error),
    NoPid,
    DatabaseError(surrealdb::Error),
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