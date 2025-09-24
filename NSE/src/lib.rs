use log::info;
use mlua::{Error as LuaError, Function, Lua};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

mod modules;
use modules::make_modules;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum InputType {
    Number {
        default: Option<i64>,
        min: Option<i64>,
        max: Option<i64>,
        value: i64,
    },
    Text {
        default: Option<String>,
        regex: Option<String>,
        value: String,
    },
    Boolean {
        default: Option<bool>,
        value: bool,
    },
    Choice {
        default: Option<String>,
        values: Vec<String>,
        value: String,
    },
}

pub struct NinjaEngine {
    lua: Lua,
}

impl NinjaEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();

        let globals = lua.globals();

        let (fs, env, shell, time, json, http, log) = make_modules(&lua)?;

        globals.set("fs", fs)?;
        globals.set("env", env)?;
        globals.set("shell", shell)?;
        globals.set("time", time)?;
        globals.set("json", json)?;
        globals.set("http", http)?;
        globals.set("log", log)?;

        let engine = Self { lua };
        Ok(engine)
    }

    pub fn execute(&self, script: &str) -> Result<(), LuaError> {
        info!("Executing lua script.");
        self.lua.load(script).exec()
    }

    pub fn execute_file(&self, path: &str) -> Result<(), LuaError> {
        info!("Executing file: {}", path);

        let script = fs::read_to_string(path)?;

        self.lua.load(script).exec()
    }

    pub fn execute_function(&self, function: &str, path: &PathBuf) -> Result<(), LuaError> {
        let globals = self.lua.globals();

        let script = std::fs::read_to_string(path)?;

        // First try evaluating the script and capturing its return value
        let value = self.lua.load(&script).eval::<mlua::Value>()?;

        // Try to get the function from return value if it's a table
        let func: Function = match value {
            mlua::Value::Table(table) => table.get(function)?,
            _ => {
                // Fall back to looking in globals
                globals.get(function)?
            }
        };

        func.call::<()>(())
    }
}
