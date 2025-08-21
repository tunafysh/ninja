use std::{fs, path::PathBuf, collections::HashMap};
use log::info;
use serde::{Serialize, Deserialize};
use mlua::{Error as LuaError, Function, Lua};
use crate::{modules::make_modules};

mod modules;

// copying the same struct from API bc i am too young to deal with compiler race conditions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigParam {
    pub input: InputType,
    pub script: String,
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

pub struct NinjaEngine {
    lua: Lua
}

impl NinjaEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();

        let (fs, env, shell, time, json, http, log) = make_modules(&lua)?;

        let globals = lua.globals();

        globals.set("fs", fs)?;
        globals.set("env", env)?;
        globals.set("shell", shell)?;
        globals.set("time", time)?;
        globals.set("json", json)?;
        globals.set("http", http)?;
        globals.set("log", log)?;

        let engine = Self { lua: lua };
        Ok(engine)
    }

    pub fn execute(&self, script: &str, context: Option<HashMap<String, ConfigParam>>) -> Result<(), LuaError> {
        info!("Executing lua script.");
        self.lua.load(script).exec()
    }

    pub fn execute_file(&self, path: &str, context: Option<HashMap<String, ConfigParam>>) -> Result<(), LuaError> {
        info!("Executing file: {}", path);

        let script = fs::read_to_string(path)?;

        self.lua.load(script).exec()
    }

    pub fn execute_function(&self, function: &str, file: &PathBuf, context: Option<HashMap<String, ConfigParam>>) -> Result<(), LuaError> {
        let script = fs::read_to_string(file)?;

        
        
        self.lua.load(script).exec()?;
        let func: Function = self.lua.globals().get(function).expect("Failed to get start function");
        func.call::<()>(())
    }
}