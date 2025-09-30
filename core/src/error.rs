// Simplified error handling with automatic conversions
#[derive(Debug)]
pub enum ShurikenError {
    ServiceNotFound(String),
    SpawnFailed(String, std::io::Error),
    NoPid,
    ConfigError(String),
    ShurikensDirectoryNotFound,
    InvalidServiceName,
    ConfigParseError(String, toml::de::Error),
    IoError(std::io::Error),
}

impl std::fmt::Display for ShurikenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShurikenError::ServiceNotFound(name) => write!(f, "Service '{}' not found", name),
            ShurikenError::SpawnFailed(name, err) => {
                write!(f, "Failed to spawn service '{}': {}", name, err)
            }
            ShurikenError::NoPid => write!(f, "Could not get process ID"),
            ShurikenError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ShurikenError::ShurikensDirectoryNotFound => write!(f, "Shurikens directory not found"),
            ShurikenError::InvalidServiceName => write!(f, "Invalid service name"),
            ShurikenError::ConfigParseError(name, err) => {
                write!(f, "Failed to parse config for '{}': {}", name, err)
            }
            ShurikenError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ShurikenError {}

// Automatic error conversions
impl From<std::io::Error> for ShurikenError {
    fn from(err: std::io::Error) -> Self {
        ShurikenError::IoError(err)
    }
}
