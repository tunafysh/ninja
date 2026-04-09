pub mod dsl;
pub mod templater;

mod modules;
use crate::{manager::ShurikenManager, utils::resolve_path};
use log::info;
use mlua::{Error as LuaError, Lua};
use modules::{
    make_env_module, make_fs_module, make_modules, make_ninja_module, make_proc_module,
    make_shell_module,
};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct NinjaEngine {
    preload_dir: Option<PathBuf>,
    #[cfg(feature = "testing")]
    pub lua: Lua,
    #[cfg(not(feature = "testing"))]
    lua: Lua,
}

impl NinjaEngine {
    fn load_preloads(&self) -> Result<(), LuaError> {
        let Some(dir) = &self.preload_dir else {
            return Ok(());
        };

        let lua = &self.lua;
        let globals = lua.globals();

        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();

            if path.extension().and_then(|s| s.to_str()) != Some("ns") {
                continue;
            }

            let re = Regex::new(r"export\s+function\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .map_err(|e| LuaError::RuntimeError(e.to_string()))?;

            let name = path.file_stem().unwrap().to_string_lossy();
            let raw_script =
                std::fs::read_to_string(&path)?.replace("export function", "function __mod.");
            let transformed = re.replace_all(&raw_script, "function __mod.$1").to_string();
            let script = format!("local __mod = {{}}; {}\nreturn __mod", transformed);

            let module: mlua::Table = lua.load(&script).eval()?;

            globals.set(name.as_ref(), module)?;
        }

        Ok(())
    }
    /// Default constructor: modules are created with no fixed cwd.
    /// All existing call sites can keep using this.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new_with(mlua::StdLib::ALL_SAFE, mlua::LuaOptions::default())?;
        let globals = lua.globals();

        let (fs, env, shell, time, json, http, log, proc) = make_modules(&lua, None).await?;

        globals.set("fs", fs)?;
        globals.set("env", env)?;
        globals.set("shell", shell)?;
        globals.set("time", time)?;
        globals.set("json", json)?;
        globals.set("http", http)?;
        globals.set("log", log)?;
        globals.set("proc", proc)?;

        let engine = Self {
            lua,
            preload_dir: Some(PathBuf::from(".ninja/preloads")),
        };

        engine.load_preloads()?;

        Ok(engine)
    }

    /// Execute a raw Lua script in the global environment.
    pub fn execute(
        &self,
        script: &str,
        cwd: Option<&Path>,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), LuaError> {
        let globals = self.lua.globals();

        if let Some(mgr) = mgr {
            let ninja = make_ninja_module(&self.lua, mgr)?;
            globals.set("ninja", ninja)?;
        }

        let fs = make_fs_module(&self.lua, cwd)?;
        let env = make_env_module(&self.lua, cwd)?;
        let shell = make_shell_module(&self.lua, cwd)?;
        let proc = make_proc_module(&self.lua, cwd)?;
        globals.set("fs", fs)?;
        globals.set("env", env)?;
        globals.set("shell", shell)?;
        globals.set("proc", proc)?;

        info!("Executing lua script.");
        self.lua.load(script).exec()
    }

    /// Execute a file in the global environment, resolving path optionally against `cwd`.
    pub fn execute_file(
        &self,
        path: &PathBuf,
        cwd: Option<&Path>,
        mgr: Option<ShurikenManager>,
    ) -> Result<(), LuaError> {
        info!("Executing file: {:#?}", path);
        let globals = self.lua.globals();

        if let Some(mgr) = mgr {
            let ninja = make_ninja_module(&self.lua, mgr)?;
            globals.set("ninja", ninja)?;
        }

        let script = if let Some(cwd) = cwd {
            let fs = make_fs_module(&self.lua, Some(cwd))?;
            let env = make_env_module(&self.lua, Some(cwd))?;
            let shell = make_shell_module(&self.lua, Some(cwd))?;
            let proc = make_proc_module(&self.lua, Some(cwd))?;
            globals.set("fs", fs)?;
            globals.set("env", env)?;
            globals.set("shell", shell)?;
            globals.set("proc", proc)?;
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
        mgr: Option<ShurikenManager>,
    ) -> Result<(), LuaError> {
        let lua = &self.lua;
        let globals = self.lua.globals();

        if let Some(mgr) = mgr {
            let ninja = make_ninja_module(&self.lua, mgr)?;
            globals.set("ninja", ninja)?;
        }

        let script = if let Some(cwd) = cwd {
            let fs = make_fs_module(&self.lua, Some(cwd))?;
            let env = make_env_module(&self.lua, Some(cwd))?;
            let shell = make_shell_module(&self.lua, Some(cwd))?;
            let proc = make_proc_module(&self.lua, Some(cwd))?;
            globals.set("fs", fs)?;
            globals.set("env", env)?;
            globals.set("shell", shell)?;
            globals.set("proc", proc)?;
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

        // Execute and capture the return value
        let result: mlua::Value = chunk.eval()?;

        // Try to get the function from the returned value first (if it's a table)
        let func: mlua::Function = if let mlua::Value::Table(table) = result {
            // Script returned a table, try to get the function from it
            table.get(function)?
        } else {
            // Script didn't return a table, try to get the function from the environment
            env.get(function)?
        };

        // Call the function with no arguments
        func.call::<()>(())
    }
}
