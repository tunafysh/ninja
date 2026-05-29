/// Error types specific to Shuriken operations.
///
/// Provides detailed error information for various failure scenarios
/// when working with Shurikens and configuration.
#[derive(Debug)]
pub enum ShurikenError {
    /// A requested Shuriken service was not found
    ServiceNotFound(String),
    /// Failed to spawn a process for a service
    SpawnFailed(String, std::io::Error),
    /// Could not retrieve the process ID
    NoPid,
    /// Configuration-related error
    ConfigError(String),
    /// The Shurikens directory does not exist
    ShurikensDirectoryNotFound,
    /// The provided service name is invalid
    InvalidServiceName,
    /// Failed to parse configuration from TOML
    ConfigParseError(String, toml::de::Error),
    /// I/O operation failed
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
