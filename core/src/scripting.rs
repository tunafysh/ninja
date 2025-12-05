use super::modules::make_modules;
use crate::{modules::{make_env_module, make_fs_module, make_shell_module}, util::resolve_path};
use log::info;
use mlua::{Error as LuaError, Lua};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct NinjaEngine {
    #[cfg(feature = "testing")]
    pub lua: Lua,
    #[cfg(not(feature = "testing"))]
    lua: Lua,
}

impl NinjaEngine {
    /// Default constructor: modules are created with no fixed cwd.
    /// All existing call sites can keep using this.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();
        let globals = lua.globals();

        // NOTE: this is the adapted call: pass cwd down to make_modules
        let (fs, env, shell, time, json, http, log, proc) = make_modules(&lua, None).await?;

        globals.set("fs", fs)?;
        globals.set("env", env)?;
        globals.set("shell", shell)?;
        globals.set("time", time)?;
        globals.set("json", json)?;
        globals.set("http", http)?;
        globals.set("log", log)?;
        globals.set("proc", proc)?;

        Ok(Self { lua })
    }

    /// Execute a raw Lua script in the global environment.
    pub fn execute(&self, script: &str) -> Result<(), LuaError> {
        info!("Executing lua script.");
        self.lua.load(script).exec()
    }

    /// Execute a file in the global environment, resolving path optionally against `cwd`.
    pub fn execute_file(&self, path: &PathBuf, cwd: Option<&Path>) -> Result<(), LuaError> {
        info!("Executing file: {:#?}", path);

        let script = if let Some(cwd) = cwd {
            let globals = self.lua.globals();
            let fs = make_fs_module(&self.lua, Some(cwd))?;
            let env = make_env_module(&self.lua, Some(cwd))?;
            let shell = make_shell_module(&self.lua, Some(cwd))?;
            globals.set("fs", fs)?;
            globals.set("env", env)?;
            globals.set("shell", shell)?;
            fs::read_to_string(resolve_path(cwd, path))?
        } else {
            fs::read_to_string(path)?
        };

        self.lua.load(script).exec()
    }

    /// Execute a specific function from a script in an isolated environment.
    /// The script is loaded from `path` (optionally resolved against `cwd`),
    /// its globals live in a fresh env that inherits from `lua.globals()`,
    /// and then `function` is retrieved from that env and called.
    pub fn execute_function(
        &self,
        function: &str,
        path: &PathBuf,
        cwd: Option<&Path>,
    ) -> Result<(), LuaError> {
        let lua = &self.lua;

        let script = if let Some(cwd) = cwd {
            let globals = self.lua.globals();
            let fs = make_fs_module(&self.lua, Some(cwd))?;
            let env = make_env_module(&self.lua, Some(cwd))?;
            let shell = make_shell_module(&self.lua, Some(cwd))?;
            globals.set("fs", fs)?;
            globals.set("env", env)?;
            globals.set("shell", shell)?;
            fs::read_to_string(resolve_path(cwd, path))?
        } else {
            fs::read_to_string(path)?
        };

        // Create isolated env for the script
        let env = lua.create_table()?;

        // Make environment inherit standard globals
        let globals = lua.globals();
        env.set_metatable(Some(lua.create_table_from([("__index", globals)])?))?;

        // Load script into the isolated environment
        let chunk = lua.load(&script).set_environment(env.clone());

        // Execute once so the script can define functions into `env`
        chunk.exec()?;

        // Now extract the function from the isolated environment
        let func: mlua::Function = env.get(function)?;

        // Call start()/stop() with no arguments for now
        func.call::<()>(())
    }
}
