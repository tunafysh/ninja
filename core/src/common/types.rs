use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use toml::{Value, map::Map};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShurikenState {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "idle")]
    Idle,
    Error(String),
}

// Unified platform-aware path type
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FieldValue {
    String(String),
    Number(i64),
    Bool(bool),
    Map(HashMap<String, FieldValue>),
    Array(Vec<FieldValue>), // If you want later: Float(f64), List(Vec<FieldValue>), Datetime(String), etc.
}

impl FieldValue {
    /// Recursively lookup a dotted path: e.g. `config.ssl.port`
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

    /// Render value as string (for template substitution)
    pub fn render(&self) -> String {
        match self {
            FieldValue::String(s) => s.clone(),
            FieldValue::Number(n) => n.to_string(),
            FieldValue::Bool(b) => b.to_string(),
            FieldValue::Array(a) => format!("{:#?}", a).to_string(),
            FieldValue::Map(_) => "[object map]".to_string(),
        }
    }

    /// Convenience casts
    pub fn as_str(&self) -> Option<&str> {
        if let FieldValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let FieldValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArmoryMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub synopsis: Option<String>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub postinstall: Option<PathBuf>,
    pub platform: String,
}
