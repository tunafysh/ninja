use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeStatus {
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
    pub directory_name: String,
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