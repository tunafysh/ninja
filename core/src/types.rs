use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShurikenState {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "idle")]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Value {
    String(String),
    Number(i64),
    Bool(bool),
    Map(HashMap<String, Value>),
}

impl Value {
    /// Recursively lookup a dotted path: e.g. `config.ssl.port`
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let mut current = self;
        for part in path.split('.') {
            match current {
                Value::Map(map) => {
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
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Map(_) => "[object map]".to_string(), // you usually don’t want maps inline
        }
    }
}

impl From<&str> for Value {
    fn from(val: &str) -> Self {
        let val = val.trim();

        if let Ok(n) = val.parse::<i64>() {
            Value::Number(n)
        } else if val.eq_ignore_ascii_case("true") {
            Value::Bool(true)
        } else if val.eq_ignore_ascii_case("false") {
            Value::Bool(false)
        } else if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            Value::String(val[1..val.len() - 1].to_string())
        } else {
            Value::String(val.to_string())
        }
    }
}

impl From<String> for Value {
    fn from(val: String) -> Self {
        Value::from(val.as_str())
    }
}

impl From<toml::Value> for Value {
    fn from(val: toml::Value) -> Self {
        match val {
            toml::Value::String(s) => Value::String(s),
            toml::Value::Integer(i) => Value::Number(i),
            toml::Value::Boolean(b) => Value::Bool(b),
            toml::Value::Table(table) => {
                let mut map = HashMap::new();
                for (k, v) in table {
                    map.insert(k, Value::from(v));
                }
                Value::Map(map)
            }
            toml::Value::Array(arr) => {
                // could add a `List(Vec<Value>)` variant if needed
                let mut map = HashMap::new();
                for (i, v) in arr.into_iter().enumerate() {
                    map.insert(i.to_string(), Value::from(v));
                }
                Value::Map(map)
            }
            toml::Value::Float(f) => {
                // you only have i64 — up to you if you want to extend Value with Float
                Value::String(f.to_string())
            }
            toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        }
    }
}