use super::modules::make_modules;
use log::info;
use mlua::{Error as LuaError, Lua};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct NinjaEngine {
    #[cfg(feature = "testing")]
    pub lua: Lua,
    #[cfg(not(feature = "testing"))]
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

    pub fn execute_file(&self, path: &PathBuf) -> Result<(), LuaError> {
        info!("Executing file: {:#?}", path);

        let script = fs::read_to_string(path)?;

        self.lua.load(script).exec()
    }

    pub fn execute_function(&self, function: &str, path: &PathBuf) -> Result<(), LuaError> {
        let lua = &self.lua;
    
        let script = std::fs::read_to_string(path)?;
    
        // Create isolated env for the script
        let env = lua.create_table()?;
    
        // Make environment inherit standard functions (optional)
        let globals = lua.globals();
        env.set_metatable(Some(lua.create_table_from([("__index", globals)])?))?;
    
        // Load script into the isolated environment
        let chunk = lua.load(&script).set_environment(env.clone());
    
        // Execute only once, into env, NOT global
        chunk.exec()?; 
    
        // Now extract the function from the isolated environment
        let func: mlua::Function = env.get(function)?;
    
        func.call::<()>(())
    }

}
