use chrono::prelude::*;
use log::{debug, error, info, warn};
use mlua::{Lua, LuaSerdeExt, Result, Table};
use runas::Command as AdminCmd;
use crate::shuriken::{kill_process_by_pid, kill_process_by_name};
use serde_json::Value;

use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output, Stdio},
    sync::Mutex,
    time::Duration,
};

lazy_static::lazy_static! {
    static ref DETACHED_PROCS: Mutex<HashMap<u32, std::process::Child>> = Mutex::new(HashMap::new());
}

#[cfg(windows)]
fn run_windows_command(command: &str) -> Result<Output> {
    Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-WindowStyle").arg("Hidden")
        .arg("-Command").arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| mlua::Error::external(e))
}

#[cfg(unix)]
fn run_unix_command(command: &str) -> Result<Output> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    Command::new(shell)
        .args(["-c", command])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| mlua::Error::external(e))
}

pub fn make_modules(lua: &Lua) -> Result<(Table, Table, Table, Table, Table, Table, Table)> {
    let fs_module = lua.create_table()?;
    let env_module = lua.create_table()?;
    let shell_module = lua.create_table()?;
    let time_module = lua.create_table()?;
    let json_module = lua.create_table()?;
    let http_module = lua.create_table()?;
    let log_module = lua.create_table()?;

    // ====== FS module ======
    fs_module.set("read", lua.create_function(|_, path: String| Ok(fs::read_to_string(path)?))?)?;
    fs_module.set("write", lua.create_function(|_, (path, content): (String, String)| Ok(fs::write(path, content)?))?)?;
    fs_module.set("append", lua.create_function(|_, (path, content): (String, String)| {
        let mut file = fs::OpenOptions::new().append(true).open(path)?;
        Ok(file.write_all(content.as_bytes())?)
    })?)?;
    fs_module.set("remove", lua.create_function(|_, path: String| Ok(fs::remove_file(path)?))?)?;
    fs_module.set("create_dir", lua.create_function(|_, path: String| Ok(fs::create_dir(path)?))?)?;
    fs_module.set("read_dir", lua.create_function(|_, path: String| {
        Ok(fs::read_dir(path)?
            .map(|e| e.unwrap().file_name().into_string().unwrap_or_else(|os| format!("<invalid UTF-8: {:?}>", os)))
            .collect::<Vec<String>>())
    })?)?;
    fs_module.set("exists", lua.create_function(|_, path: String| Ok(fs::metadata(&path).is_ok()))?)?;
    fs_module.set("is_dir", lua.create_function(|_, path: String| Ok(Path::new(&path).is_dir()))?)?;
    fs_module.set("is_file", lua.create_function(|_, path: String| Ok(Path::new(&path).is_file()))?)?;

    // ====== ENV module ======
    env_module.set("os", env::consts::OS)?;
    env_module.set("arch", env::consts::ARCH)?;
    env_module.set("get", lua.create_function(|_, key: String| Ok(env::var(key).ok()))?)?;
    env_module.set("set", lua.create_function(|_, (key, value): (String, String)| unsafe { env::set_var(key, value); Ok(()) })?)?;
    env_module.set("remove", lua.create_function(|_, key: String| unsafe { env::remove_var(key); Ok(()) })?)?;
    env_module.set("vars", lua.create_function(|lua, _: ()| {
        let table = lua.create_table()?;
        for (k, v) in env::vars() { table.set(k, v)?; }
        Ok(table)
    })?)?;
    env_module.set("cwd", lua.create_function(|_, _: ()| Ok(env::current_dir()?))?)?;
    env_module.set("kill_pid", lua.create_function(|_, pid: u32| Ok(kill_process_by_pid(pid)))?)?;
    env_module.set("kill_name", lua.create_function(|_, name: String| Ok(kill_process_by_name(&name)))?)?;

    // ====== SHELL module ======
    shell_module.set("exec", lua.create_function(|lua, (command, detached, admin): (String, Option<bool>, Option<bool>)| {
        let detached = detached.unwrap_or(false);
        let admin = admin.unwrap_or(false);
        let result_table = lua.create_table()?;
    
        if detached {
            #[cfg(windows)]
            let child = Command::new("powershell.exe")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(&command)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .map_err(|e| mlua::Error::external(e))?;
    
            #[cfg(unix)]
            let child = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| mlua::Error::external(e))?;
    
            let pid = child.id();
            DETACHED_PROCS.lock().unwrap().insert(pid, child);
            result_table.set("pid", pid)?;
        } else if admin {
            #[cfg(windows)]
            let status = AdminCmd::new("powershell.exe")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(&command)
                .show(false)
                .status()
                .map_err(|e| mlua::Error::external(e))?;
    
            #[cfg(not(windows))]
            let status = AdminCmd::new(command)
                .show(false)
                .status()
                .map_err(|e| mlua::Error::external(e))?;
    
            if let Some(code) = status.code() { result_table.set("code", code)?; }
        } else {
            let output: Result<Output> = {
                #[cfg(windows)] { run_windows_command(&command) }
                #[cfg(unix)] { run_unix_command(&command) }
            };
    
            match output {
                Ok(cmd_output) => {
                    result_table.set("code", cmd_output.status.code().unwrap_or(-1))?;
                    result_table.set("stdout", String::from_utf8_lossy(&cmd_output.stdout).to_string())?;
                    result_table.set("stderr", String::from_utf8_lossy(&cmd_output.stderr).to_string())?;
                }
                Err(e) => {
                    result_table.set("code", -1)?;
                    result_table.set("stdout", "")?;
                    result_table.set("stderr", format!("Failed: {}", e))?;
                }
            }
        }
    
        Ok(result_table)
    })?)?;

    shell_module.set("kill_pid", lua.create_function(|_, pid: u32| {
        if let Some(mut child) = DETACHED_PROCS.lock().unwrap().remove(&pid) {
            child.kill().map_err(|e| mlua::Error::external(e))?;
            Ok(true)
        } else { Ok(false) }
    })?)?;

    // ====== TIME module ======
    time_module.set("year", lua.create_function(|_, _: ()| Ok(Utc::now().year()))?)?;
    time_module.set("month", lua.create_function(|_, _: ()| Ok(Utc::now().month()))?)?;
    time_module.set("day", lua.create_function(|_, _: ()| Ok(Utc::now().day()))?)?;
    time_module.set("hour", lua.create_function(|_, format: bool| {
        let now = Utc::now();
        if format {
            let (pm, hour) = now.hour12();
            Ok((hour, if pm { "PM" } else { "AM" }))
        } else { Ok((now.hour(), "")) }
    })?)?;
    time_module.set("minute", lua.create_function(|_, _: ()| Ok(Utc::now().minute()))?)?;
    time_module.set("second", lua.create_function(|_, _: ()| Ok(Utc::now().second()))?)?;
    time_module.set("now", lua.create_function(|_, fmt: String| Ok(Utc::now().format(&fmt).to_string()))?)?;
    time_module.set("sleep", lua.create_function(|_, seconds: f64| { std::thread::sleep(Duration::from_secs_f64(seconds)); Ok(()) })?)?;

    // ====== JSON module ======
    json_module.set("encode", lua.create_function(|_, table: Table| {
        serde_json::to_string(&table).map_err(|e| e.into_lua_err())
    })?)?;
    json_module.set("decode", lua.create_function(|lua, s: String| {
        let val: Value = serde_json::from_str(&s).unwrap_or(Value::String(format!("Invalid JSON: {}", s)));
        match lua.to_value(&val) {
            mlua::Value::Table(t) => Ok(t),
            _ => Err(mlua::Error::DeserializeError("Expected JSON object/array".into())),
        }
    })?)?;

    // ====== LOG module ======
    log_module.set("info", lua.create_function(|_, s: String| { info!("{}", s); Ok(()) })?)?;
    log_module.set("warn", lua.create_function(|_, s: String| { warn!("{}", s); Ok(()) })?)?;
    log_module.set("error", lua.create_function(|_, s: String| { error!("{}", s); Ok(()) })?)?;
    log_module.set("debug", lua.create_function(|_, s: String| { debug!("{}", s); Ok(()) })?)?;

    Ok((fs_module, env_module, shell_module, time_module, json_module, http_module, log_module))
}
