use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use toml::{Value, map::Map};

/// Represents the runtime state of a Shuriken.
///
/// - `Running`: The Shuriken's process is actively running
/// - `Idle`: The Shuriken is stopped or has never been started
/// - `Error(String)`: The Shuriken encountered an error with the provided message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShurikenState {
    /// Actively running
    Running,
    /// Stopped or not started
    Idle,
    /// Error state with error message
    Error(String),
}

impl Default for ShurikenState {
    /// Default state is `Idle`
    fn default() -> Self {
        ShurikenState::Idle
    }
}

/// Platform-aware path that can be different for Windows and Unix systems.
///
/// Allows specifying platform-specific paths in configuration files.
/// The `get_path()` method returns the appropriate path for the current platform.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum PlatformPath {
    /// A simple path used on all platforms
    Simple(String),
    /// Platform-specific paths (Windows vs Unix)
    Platform { windows: String, unix: String },
}

impl PlatformPath {
    /// Returns the path string appropriate for the current platform.
    ///
    /// # Returns
    /// - The Windows path if compiled for Windows
    /// - The Unix path otherwise
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

/// A flexible value type used for configuration options.
///
/// Supports strings, integers, booleans, maps, and arrays.
/// Provides methods for accessing and converting values.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FieldValue {
    /// A string value
    String(String),
    /// An integer value
    Number(i64),
    /// A boolean value
    Bool(bool),
    /// A map of field values (for nested configuration)
    Map(HashMap<String, FieldValue>),
    /// An array of field values
    Array(Vec<FieldValue>),
}

impl FieldValue {
    /// Recursively looks up a value using a dotted path (e.g., `config.ssl.port`).
    ///
    /// # Arguments
    /// - `path`: A dot-separated path to traverse the nested structure
    ///
    /// # Returns
    /// - `Some(&FieldValue)` if the path exists
    /// - `None` if the path doesn't exist or a non-map is encountered
    pub fn get_path(&self, path: &str) -> Option<&FieldValue> {
        let mut current = self;
        for part in path.split('.') {
            match current {
                FieldValue::Map(map) => {
                    current = map.get(part)?;
                }
                _ => return None, // tried to go deeper but hit a scalar
            }
        }
        Some(current)
    }

    /// Renders the value as a string for template substitution.
    ///
    /// - Strings and numbers are converted directly
    /// - Booleans become "true" or "false"
    /// - Arrays are debug-formatted
    /// - Maps become "[object map]"
    pub fn render(&self) -> String {
        match self {
            FieldValue::String(s) => s.clone(),
            FieldValue::Number(n) => n.to_string(),
            FieldValue::Bool(b) => b.to_string(),
            FieldValue::Array(a) => format!("{:#?}", a).to_string(),
            FieldValue::Map(_) => "[object map]".to_string(),
        }
    }

    /// Attempts to extract a string value.
    ///
    /// # Returns
    /// - `Some(&str)` if this is a `String` variant
    /// - `None` otherwise
    pub fn as_str(&self) -> Option<&str> {
        if let FieldValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Attempts to extract an integer value.
    ///
    /// # Returns
    /// - `Some(i64)` if this is a `Number` variant
    /// - `None` otherwise
    pub fn as_int(&self) -> Option<i64> {
        if let FieldValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Attempts to extract a boolean value.
    ///
    /// # Returns
    /// - `Some(bool)` if this is a `Bool` variant
    /// - `None` otherwise
    pub fn as_bool(&self) -> Option<bool> {
        if let FieldValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

// ---------------- From impls ----------------

impl From<&str> for FieldValue {
    fn from(val: &str) -> Self {
        let val = val.trim();

        // Try number
        if let Ok(n) = val.parse::<i64>() {
            return FieldValue::Number(n);
        }

        // Try bool
        if val.eq_ignore_ascii_case("true") {
            return FieldValue::Bool(true);
        }
        if val.eq_ignore_ascii_case("false") {
            return FieldValue::Bool(false);
        }

        // Try string with quotes
        if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            return FieldValue::String(val[1..val.len() - 1].to_string());
        }

        // Fallback
        FieldValue::String(val.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(val: String) -> Self {
        FieldValue::from(val.as_str())
    }
}

impl From<toml::Value> for FieldValue {
    fn from(val: toml::Value) -> Self {
        match val {
            toml::Value::String(s) => FieldValue::String(s),
            toml::Value::Integer(i) => FieldValue::Number(i),
            toml::Value::Boolean(b) => FieldValue::Bool(b),
            toml::Value::Table(table) => {
                let map = table
                    .into_iter()
                    .map(|(k, v)| (k, FieldValue::from(v)))
                    .collect();
                FieldValue::Map(map)
            }
            toml::Value::Array(a) => {
                // For now, store arrays as indexed maps
                let mut arr: Vec<FieldValue> = Vec::new();

                for i in a {
                    arr.push(FieldValue::from(i));
                }

                FieldValue::Array(arr)
            }
            toml::Value::Float(f) => FieldValue::String(f.to_string()),
            toml::Value::Datetime(dt) => FieldValue::String(dt.to_string()),
        }
    }
}

impl From<FieldValue> for toml::Value {
    fn from(f: FieldValue) -> Self {
        match f {
            FieldValue::String(s) => Value::String(s.clone()),
            FieldValue::Number(n) => Value::Integer(n),
            FieldValue::Bool(b) => Value::Boolean(b),
            FieldValue::Array(a) => {
                let mut arr: Vec<Value> = Vec::new();

                for i in a.clone() {
                    arr.push(i.into());
                }

                Value::Array(arr)
            }
            FieldValue::Map(m) => {
                let mut map: Map<String, Value> = Map::new();

                for (name, value) in m {
                    map.insert(name.clone(), value.clone().into());
                }

                Value::Table(map)
            }
        }
    }
}

/// Metadata for a packaged Shuriken or bundle in the Armory.
///
/// Contains information about the Shuriken including identity, licensing,
/// authorship, and postinstall scripts.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArmoryMetadata {
    /// Unique identifier for the package
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string (e.g., "1.0.0")
    pub version: String,
    /// Short description of the package
    pub synopsis: Option<String>,
    /// Detailed description
    pub description: Option<String>,
    /// List of authors
    pub authors: Option<Vec<String>>,
    /// License identifier (e.g., "MIT", "Apache-2.0")
    pub license: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Path to postinstall script
    pub postinstall: Option<PathBuf>,
    /// Supported platforms (e.g., "linux-x86_64,windows-x86_64")
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LocalInstallStages {
    Validating,
    Extracting(u8),
    PostInstall,
    Installed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstallStage {
    Downloading,
    Validating,
    Extracting,
    PostInstall,
    Installed,
}

#[derive(Debug, Clone)]
pub struct InstallEvent {
    pub stage: InstallStage,
    pub progress: u8,
}
