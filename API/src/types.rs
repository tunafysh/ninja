use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShurikenState {
    #[serde(rename= "running")]
    Running,
    #[serde(rename= "idle")]
    Idle,
    Error(String),
}

// Unified platform-aware path type
#[derive(Debug, Serialize, Deserialize, Clone)]
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