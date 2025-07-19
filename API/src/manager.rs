use crate::config::{ServiceConfig, MaintenanceType};
use crate::error::ServiceError;
use crate::types::{RuntimeStatus, ServiceState};
use ninja_engine::NinjaEngine;
use std::collections::HashMap;
use std::{env, fs};
use log::info;
use std::path::{Path, PathBuf};
use tokio::process::Child;

pub struct ServiceManager {
    running_processes: HashMap<String, Child>,
    service_configs: HashMap<String, ServiceConfig>,
    service_states: HashMap<String, ServiceState>, // In-memory state tracking
}

impl ServiceManager {
    pub fn bootstrap() -> Result<Self, ServiceError> {
        let args = env::args().collect::<Vec<String>>();
        let exe_path = PathBuf::from(&args[0]);
        let path = exe_path.parent().ok_or_else(|| {
            ServiceError::ConfigError("Failed to determine current directory".to_string())
        })?;
        info!("Current directory set to: {}", &path.display());
        env::set_current_dir(&path)?;
        
        // Initialize service states from configs
        let configs = Self::load_service_configs()?;
        let mut service_states = HashMap::new();
        for (directory_name, config) in &configs {
            service_states.insert(
                config.shuriken.name.clone(),
                ServiceState {
                    name: config.shuriken.name.clone(),
                    status: "stopped".to_string(),
                    pid: None,
                    directory_name: directory_name.clone(),
                },
            );
        }

        Ok(Self {
            running_processes: HashMap::new(),
            service_configs: configs,
            service_states,
        })
    }

    pub async fn start_service(&mut self, service_name: &str) -> Result<u32, ServiceError> {
        let config = self.find_service_by_name(service_name)?.clone();
        let directory_name = self.find_directory_by_name(service_name)?;
        let pid = self.spawn_process(&directory_name, &config).await?;
        println!("Starting shuriken {}...", service_name);

        self.update_service_status(service_name, "running", Some(pid));
        Ok(pid)
    }

    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        let directory_name = self.find_directory_by_name(service_name)?;

        // Stop process if we're managing it
        if let Some(mut child) = self.running_processes.remove(&directory_name) {
            let _ = child.kill().await;
        }

        // Also kill by PID from in-memory state
        if let Some(state) = self.service_states.get(service_name) {
            if let Some(pid) = state.pid {
                Self::kill_process_by_pid(pid);
            }
        }

        self.update_service_status(service_name, "stopped", None);
        Ok(())
    }

    pub async fn get_all_services(&mut self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        self.cleanup_stale_processes().await;

        let services = self
            .service_states
            .values()
            .map(|state| RuntimeStatus {
                name: state.name.clone(),
                status: state.status.clone(),
                pid: state.pid,
            })
            .collect();

        Ok(services)
    }

    pub async fn get_running_services(&mut self) -> Result<Vec<RuntimeStatus>, ServiceError> {
        self.cleanup_stale_processes().await;

        let services = self
            .service_states
            .values()
            .filter(|state| state.status == "running")
            .filter_map(|state| {
                if let Some(pid) = state.pid {
                    if Self::is_process_running(pid) {
                        Some(RuntimeStatus {
                            name: state.name.clone(),
                            status: state.status.clone(),
                            pid: state.pid,
                        })
                    } else {
                        // Process died, update status
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

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

    pub fn get_service_status(&self, service_name: &str) -> Option<RuntimeStatus> {
        self.service_states.get(service_name).map(|state| RuntimeStatus {
            name: state.name.clone(),
            status: state.status.clone(),
            pid: state.pid,
        })
    }

    // Private helper methods
    async fn cleanup_stale_processes(&mut self) {
        let service_names: Vec<String> = self.service_states.keys().cloned().collect();

        for service_name in service_names {
            if let Some(state) = self.service_states.get(&service_name).cloned() {
                if state.status == "running" {
                    if let Some(pid) = state.pid {
                        if !Self::is_process_running(pid) {
                            self.update_service_status(&service_name, "stopped", None);
                            self.running_processes.remove(&state.directory_name);
                        }
                    }
                }
            }
        }
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
            MaintenanceType::Native { bin_path, args, .. } => {
                Ok((
                    bin_path.get_path().to_string(),
                    args.clone().unwrap_or_default(),
                ))
            }
            MaintenanceType::Script { script_path, .. } => {
                self.execute_ninja_script(script_path, &config.shuriken.name)?;
                Ok((String::new(), Vec::new()))
            }
            MaintenanceType::Simple(maintenance_type) => {
                match maintenance_type.as_str() {
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
                }
            }
        }
    }

    fn execute_ninja_script(
        &self,
        script_path: &PathBuf,
        service_name: &str,
    ) -> Result<(), ServiceError> {
        let engine = NinjaEngine::new();
        env::set_current_dir(format!("shurikens/{}", service_name))?;
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

    fn update_service_status(&mut self, service_name: &str, status: &str, pid: Option<u32>) {
        if let Some(state) = self.service_states.get_mut(service_name) {
            state.status = status.to_string();
            state.pid = pid;
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
            let _ = Command::new("tasklist")
                .arg("/PID")
                .arg(pid.to_string())
                .arg("/F")
                .output();
        }
    }
}