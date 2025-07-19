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
            ServiceError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ServiceError {}

// Automatic error conversions
impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::IoError(err)
    }
}