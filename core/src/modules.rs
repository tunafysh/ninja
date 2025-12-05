use crate::{
    shuriken::{kill_process_by_name, kill_process_by_pid},
    util::resolve_path,
};
use chrono::prelude::*;
use log::{debug, error, info, warn};
use mlua::{ExternalError, Lua, LuaSerdeExt, Result, Table};
use serde_json::Value;

use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::Mutex,
    time::Duration,
};

lazy_static::lazy_static! {
    static ref DETACHED_PROCS: Mutex<HashMap<u32, std::process::Child>> = Mutex::new(HashMap::new());
}

pub fn make_fs_module(lua: &Lua, cwd: Option<&Path>) -> Result<Table> {
    let fs_module = lua.create_table()?;

    // Capture cwd as an owned PathBuf so closures can move it safely
    let base_cwd: Option<PathBuf> = cwd.map(|p| p.to_path_buf());

    // Helper: resolve a path against cwd (if present)
    fn resolve_with_cwd(base_cwd: &Option<PathBuf>, path: &PathBuf) -> PathBuf {
        if let Some(cwd) = base_cwd {
            resolve_path(cwd, path)
        } else {
            path.clone()
        }
    }

    // fs.read(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "read",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                fs::read_to_string(&path).map_err(mlua::Error::external)
            })?,
        )?;
    }

    // fs.write(path, content)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "write",
            lua.create_function(move |_, (path, content): (PathBuf, String)| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                fs::write(&path, content).map_err(mlua::Error::external)
            })?,
        )?;
    }

    // fs.append(path, content)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "append",
            lua.create_function(move |_, (path, content): (PathBuf, String)| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(mlua::Error::external)?;
                file.write_all(content.as_bytes())
                    .map_err(mlua::Error::external)?;
                Ok(())
            })?,
        )?;
    }

    // fs.remove(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "remove",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                fs::remove_file(&path).map_err(mlua::Error::external)?;
                Ok(())
            })?,
        )?;
    }

    // fs.create_dir(path) – recursive
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "create_dir",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                fs::create_dir_all(&path).map_err(mlua::Error::external)?;
                Ok(())
            })?,
        )?;
    }

    // fs.read_dir(path) -> { "file1", "file2", ... }
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "read_dir",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                let entries = fs::read_dir(&path).map_err(mlua::Error::external)?;
                let mut result = Vec::new();

                for entry in entries.flatten() {
                    let name_os = entry.file_name();
                    if let Ok(name) = name_os.into_string() {
                        result.push(name);
                    } else {
                        result.push(String::from("<invalid UTF-8>"));
                    }
                }

                Ok(result)
            })?,
        )?;
    }

    // fs.exists(path) -> bool
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "exists",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                Ok(path.exists())
            })?,
        )?;
    }

    // fs.is_dir(path) -> bool
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "is_dir",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                Ok(path.is_dir())
            })?,
        )?;
    }

    // fs.is_file(path) -> bool
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "is_file",
            lua.create_function(move |_, path: PathBuf| {
                let path = resolve_with_cwd(&fs_cwd, &path);
                Ok(path.is_file())
            })?,
        )?;
    }

    Ok(fs_module)
}

pub fn make_env_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    let env_module = lua.create_table()?;

    // ====== ENV module ======
    env_module.set("os", env::consts::OS)?;
    env_module.set("arch", env::consts::ARCH)?;
    env_module.set(
        "get",
        lua.create_function(|_, key: String| Ok(env::var(key).ok()))?,
    )?;
    env_module.set(
        "set",
        lua.create_function(|_, (key, value): (String, String)| unsafe {
            env::set_var(key, value);
            Ok(())
        })?,
    )?;
    env_module.set(
        "remove",
        lua.create_function(|_, key: String| unsafe {
            env::remove_var(key);
            Ok(())
        })?,
    )?;
    env_module.set(
        "vars",
        lua.create_function(|lua, _: ()| {
            let table = lua.create_table()?;
            for (k, v) in env::vars() {
                table.set(k, v)?;
            }
            Ok(table)
        })?,
    )?;
    {
        let cwd_string = base_cwd
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        env_module.set(
            "cwd",
            lua.create_function(move |_, _: ()| Ok(cwd_string.clone()))?,
        )?;
    }
    Ok(env_module)
}

#[cfg(windows)]
fn run_windows_command(command: &str, cwd: Option<&Path>) -> Result<Output> {
    let mut cmd = Command::new("powershell.exe");
    cmd.arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.output().map_err(mlua::Error::external)
}

#[cfg(unix)]
fn run_unix_command(command: &str, cwd: Option<&Path>) -> Result<Output> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let mut cmd = Command::new(shell);
    cmd.args(["-c", command])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.output().map_err(mlua::Error::external)
}

pub fn make_shell_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    let shell_module = lua.create_table()?;

    // Capture cwd as owned PathBuf so we can move it into the closure
    let cwd_buf: Option<PathBuf> = base_cwd.map(|p| p.to_path_buf());

    shell_module.set(
        "exec",
        lua.create_function(
            move |lua, (command, detached, admin): (String, Option<bool>, Option<bool>)| {
                let detached = detached.unwrap_or(false);
                let admin = admin.unwrap_or(false);
                let result_table = lua.create_table()?;

                let cwd_opt = cwd_buf.as_deref();

                if detached {
                    // ---------- DETACHED ----------
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;

                        let mut cmd = Command::new("powershell.exe");
                        cmd.arg("-NoProfile")
                            .arg("-WindowStyle")
                            .arg("Hidden")
                            .arg("-Command")
                            .arg(&command)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .creation_flags(0x08000000); // CREATE_NO_WINDOW

                        if let Some(cwd) = cwd_opt {
                            cmd.current_dir(cwd);
                        }

                        let child = cmd.spawn().map_err(mlua::Error::external)?;
                        let pid = child.id();
                        DETACHED_PROCS.lock().unwrap().insert(pid, child);
                        result_table.set("pid", pid)?;
                    }

                    #[cfg(unix)]
                    {
                        let mut cmd = Command::new("sh");
                        cmd.arg("-c")
                            .arg(&command)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit());

                        if let Some(cwd) = cwd_opt {
                            cmd.current_dir(cwd);
                        }

                        let child = cmd.spawn().map_err(mlua::Error::external)?;
                        let pid = child.id();
                        DETACHED_PROCS.lock().unwrap().insert(pid, child);
                        result_table.set("pid", pid)?;
                    }
                } else if admin {
                    // ---------- ADMIN ----------
                    // AdminCmd doesn't support current_dir(),
                    // so we inject a `cd` / `Set-Location` into the command string instead.

                    #[cfg(windows)]
                    {
                        let effective_command = if let Some(cwd) = cwd_opt {
                            // NOTE: naive quoting; good enough for most paths
                            format!("Set-Location '{}'; {}", cwd.display(), command)
                        } else {
                            command.clone()
                        };

                        let status = AdminCmd::new("powershell.exe")
                            .arg("-NoProfile")
                            .arg("-WindowStyle")
                            .arg("Hidden")
                            .arg("-Command")
                            .arg(&effective_command)
                            .show(false)
                            .status()
                            .map_err(mlua::Error::external)?;

                        if let Some(code) = status.code() {
                            result_table.set("code", code)?;
                        }
                    }

                    // Linux: use pkexec
                    #[cfg(target_os = "linux")]
                    {
                        let effective_command = if let Some(cwd) = cwd_opt {
                            format!("cd '{}'; {}", cwd.display(), command)
                        } else {
                            command.clone()
                        };
                    
                        let status = std::process::Command::new("pkexec")
                            .arg("sh")
                            .arg("-c")
                            .arg(&effective_command)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .status()
                            .map_err(mlua::Error::external)?;
                    
                        if let Some(code) = status.code() {
                            result_table.set("code", code)?;
                        }
                    }
                    
                    // macOS: use osascript
                    #[cfg(target_os = "macos")]
                    {
                        // cd into cwd if provided
                        let shell_cmd = if let Some(cwd) = cwd_opt {
                            format!("cd '{}'; {}", cwd.display(), command)
                        } else {
                            command.clone()
                        };
                    
                        // Very simple escaping for AppleScript string
                        fn escape_for_osascript(s: &str) -> String {
                            s.replace('\\', "\\\\").replace('"', "\\\"")
                        }
                    
                        let escaped = escape_for_osascript(&shell_cmd);
                    
                        // AppleScript: do shell script "..." with administrator privileges
                        let applescript = format!(
                            "do shell script \"{}\" with administrator privileges",
                            escaped
                        );
                    
                        let status = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(&applescript)
                            .stdin(Stdio::inherit())
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .status()
                            .map_err(mlua::Error::external)?;
                    
                        if let Some(code) = status.code() {
                            result_table.set("code", code)?;
                        }
                    }
                } else {
                    // ---------- NORMAL FOREGROUND ----------
                    let output: Result<Output> = {
                        #[cfg(windows)]
                        {
                            run_windows_command(&command, cwd_opt)
                        }
                        #[cfg(unix)]
                        {
                            run_unix_command(&command, cwd_opt)
                        }
                    };

                    match output {
                        Ok(cmd_output) => {
                            result_table.set("code", cmd_output.status.code().unwrap_or(-1))?;
                            result_table.set(
                                "stdout",
                                String::from_utf8_lossy(&cmd_output.stdout).to_string(),
                            )?;
                            result_table.set(
                                "stderr",
                                String::from_utf8_lossy(&cmd_output.stderr).to_string(),
                            )?;
                        }
                        Err(e) => {
                            result_table.set("code", -1)?;
                            result_table.set("stdout", "")?;
                            result_table.set("stderr", format!("Failed: {}", e))?;
                        }
                    }
                }

                Ok(result_table)
            },
        )?,
    )?;

    Ok(shell_module)
}

pub async fn make_modules(
    lua: &Lua,
    cwd: Option<&Path>,
) -> Result<(Table, Table, Table, Table, Table, Table, Table, Table)> {
    let fs_module = make_fs_module(lua, cwd)?;
    let env_module = make_env_module(lua, cwd)?;
    let shell_module = make_shell_module(lua, cwd)?;
    let time_module = lua.create_table()?;
    let json_module = lua.create_table()?;
    let http_module = lua.create_table()?;
    let log_module = lua.create_table()?;
    let proc_module = lua.create_table()?;

    // ====== PROC module ======
    proc_module.set(
        "spawn",
        lua.create_function(|lua, command: String| {
            let result_table = lua.create_table()?;

            #[cfg(windows)]
            use std::os::windows::process::CommandExt;
            #[cfg(windows)]
            let child = Command::new("powershell.exe")
                .arg("-NoProfile")
                .arg("-WindowStyle")
                .arg("Hidden")
                .arg("-Command")
                .arg(&command)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn()
                .map_err(mlua::Error::external)?;

            #[cfg(unix)]
            let child = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(mlua::Error::external)?;

            let pid = child.id();
            DETACHED_PROCS.lock().unwrap().insert(pid, child);
            result_table.set("pid", pid)?;

            Ok(result_table)
        })?,
    )?;

    proc_module.set(
        "kill_pid",
        lua.create_function(|_, pid: u32| {
            if let Some(mut child) = DETACHED_PROCS.lock().unwrap().remove(&pid) {
                child.kill().map_err(mlua::Error::external)?;
                Ok(true)
            } else {
                // Try system kill if not in our tracking
                Ok(kill_process_by_pid(pid))
            }
        })?,
    )?;

    proc_module.set(
        "kill_name",
        lua.create_function(|_, name: String| Ok(kill_process_by_name(&name)))?,
    )?;

    proc_module.set("list", lua.create_table()?)?;

    // ====== TIME module ======
    time_module.set(
        "year",
        lua.create_function(|_, _: ()| Ok(Utc::now().year()))?,
    )?;
    time_module.set(
        "month",
        lua.create_function(|_, _: ()| Ok(Utc::now().month()))?,
    )?;
    time_module.set("day", lua.create_function(|_, _: ()| Ok(Utc::now().day()))?)?;
    time_module.set(
        "hour",
        lua.create_function(|_, format: bool| {
            let now = Utc::now();
            if format {
                let (pm, hour) = now.hour12();
                Ok((hour, if pm { "PM" } else { "AM" }))
            } else {
                Ok((now.hour(), ""))
            }
        })?,
    )?;
    time_module.set(
        "minute",
        lua.create_function(|_, _: ()| Ok(Utc::now().minute()))?,
    )?;
    time_module.set(
        "second",
        lua.create_function(|_, _: ()| Ok(Utc::now().second()))?,
    )?;
    time_module.set(
        "now",
        lua.create_function(|_, fmt: String| Ok(Utc::now().format(&fmt).to_string()))?,
    )?;
    time_module.set(
        "sleep",
        lua.create_function(|_, seconds: f64| {
            std::thread::sleep(Duration::from_secs_f64(seconds));
            Ok(())
        })?,
    )?;

    // ====== JSON module ======
    json_module.set(
        "encode",
        lua.create_function(|_, table: Table| {
            serde_json::to_string(&table).map_err(|e| e.into_lua_err())
        })?,
    )?;
    json_module.set(
        "decode",
        lua.create_function(|lua, s: String| {
            let val: Value =
                serde_json::from_str(&s).unwrap_or(Value::String(format!("Invalid JSON: {}", s)));
            match lua.to_value(&val) {
                Ok(mlua::Value::Table(t)) => Ok(t),
                _ => Err(mlua::Error::DeserializeError(
                    "Expected JSON object/array".into(),
                )),
            }
        })?,
    )?;

    // ====== LOG module ======
    log_module.set(
        "info",
        lua.create_function(|_, s: String| {
            info!("{}", s);
            Ok(())
        })?,
    )?;
    log_module.set(
        "warn",
        lua.create_function(|_, s: String| {
            warn!("{}", s);
            Ok(())
        })?,
    )?;
    log_module.set(
        "error",
        lua.create_function(|_, s: String| {
            error!("{}", s);
            Ok(())
        })?,
    )?;
    log_module.set(
        "debug",
        lua.create_function(|_, s: String| {
            debug!("{}", s);
            Ok(())
        })?,
    )?;

    Ok((
        fs_module,
        env_module,
        shell_module,
        time_module,
        json_module,
        http_module,
        log_module,
        proc_module,
    ))
}
