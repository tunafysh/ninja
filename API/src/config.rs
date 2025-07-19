use crate::types::PlatformPath;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

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
    // These fields are needed when maintenance is a simple string
    #[serde(rename = "bin-path")]
    pub bin_path: Option<PlatformPath>,
    #[serde(rename = "script-path")]
    pub script_path: Option<PathBuf>,
    #[serde(rename = "config-path")]
    pub config_path: Option<PathBuf>,
    pub args: Option<Vec<String>>,
    #[serde(flatten, rename = "type")]
    pub shuriken_type: ShurikenType,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MaintenanceType {
    // When maintenance is just a string, we need to look for the associated fields in the parent struct
    Simple(String),
    // When maintenance is an object with specific fields
    Native {
        maintenance: String,
        #[serde(rename = "bin-path")]
        bin_path: PlatformPath,
        #[serde(rename = "config-path")]
        config_path: Option<PathBuf>,
        args: Option<Vec<String>>,
    },
    Script {
        maintenance: String,
        #[serde(rename = "script-path")]
        script_path: PathBuf,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ShurikenType {
    Simple(String),
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