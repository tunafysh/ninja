use crate::types::PlatformPath;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenConfig {
    pub name: String,
    #[serde(rename = "service-name")]
    pub service_name: String,
    pub maintenance: MaintenanceType,
    #[serde(rename = "type")]
    pub shuriken_type: String,
     #[serde(rename = "add-path")]
    pub add_path: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")] // tag field determines the variant
pub enum MaintenanceType {
    Native {
        #[serde(rename = "bin-path")]
        bin_path: PlatformPath,
        #[serde(rename = "config-path")]
        config_path: Option<PathBuf>,
        args: Option<Vec<String>>,
    },
    Script {
        #[serde(rename = "script-path")]
        script_path: PathBuf,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigParam {
    pub input: InputType,
    pub script: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogsConfig {
    #[serde(rename = "log-path")]
    pub log_path: PlatformPath,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum InputType {
    Number {
        default: Option<i64>,
        min: Option<i64>,
        max: Option<i64>,
    },
    Text {
        default: Option<String>,
        regex: Option<String>,
    },
    Boolean {
        default: Option<bool>,
    },
    Choice {
        default: Option<String>,
        values: Vec<String>,
    },
}
