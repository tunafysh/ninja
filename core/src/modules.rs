use crate::util::{kill_process_by_name, kill_process_by_pid, resolve_path};
use chrono::prelude::*;
use log::{debug, error, info, warn};
use mlua::{ExternalError, Lua, LuaSerdeExt, Result, Table};
use serde_json::Value;
use relative_path::RelativePath;

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::Duration,
};

fn canonicalize_cwd(base: Option<&Path>) -> Option<PathBuf> {
    base.map(|p| {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(p)
        };
        abs.canonicalize().unwrap_or(abs)
    })
}

fn resolve_spawn_command(command: &str, cwd: Option<&Path>) -> String {
    // Split into first token + rest
    let mut parts_iter = command.split_whitespace();
    let Some(first) = parts_iter.next() else {
        return command.to_string();
    };

    let rest: Vec<&str> = parts_iter.collect();

    // Heuristic: treat as a path if it has any slash or starts with ./ or ../
    let is_path_like =
        first.starts_with("./")
        || first.starts_with(".\\")
        || first.starts_with("../")
        || first.starts_with("..\\")
        || first.contains('/')
        || first.contains('\\');

    // If it's not path-like, keep as-is → PATH lookup etc.
    if !is_path_like {
        return command.to_string();
    }

    // If we don't have a cwd, we can't resolve logically; keep as-is
    let Some(cwd) = cwd else {
        return command.to_string();
    };

    let first_path = Path::new(first);

    // Absolute path? Just use it directly.
    let abs = if first_path.is_absolute() {
        first_path.to_path_buf()
    } else {
        // Use relative-path to apply ../, ./ relative to cwd logically
        let rel = RelativePath::new(first);
        rel.to_logical_path(cwd)
    };

    let mut cmd = abs.to_string_lossy().to_string();
    if !rest.is_empty() {
        cmd.push(' ');
        cmd.push_str(&rest.join(" "));
    }

    cmd
}

// ========================= FS MODULE =========================

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

// ========================= ENV MODULE =========================

pub fn make_env_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    let env_module = lua.create_table()?;

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

    // Canonicalized cwd string (if provided)
    let cwd_string = canonicalize_cwd(base_cwd)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    env_module.set(
        "cwd",
        lua.create_function(move |_, _: ()| Ok(cwd_string.clone()))?,
    )?;

    Ok(env_module)
}

// ========================= SHELL HELPERS =========================

#[cfg(windows)]
fn run_windows_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<Output> {
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

    // NOTE: This keeps your previous semantic (attempting elevation) even though
    //       "powershell.exe -Verb RunAs" is not really correct. You can replace
    //       this later with a proper runas/AdminCmd approach.
    if admin {
        cmd.arg("-Verb").arg("RunAs");
    }

    cmd.output().map_err(mlua::Error::external)
}

#[cfg(unix)]
fn run_unix_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<Output> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    // If admin: pkexec <shell> -c "command"
    let mut cmd = if admin {
        let mut c = Command::new("pkexec");
        c.arg("--keep-cwd");
        c.arg(&shell);
        c
    } else {
        Command::new(shell)
    };

    cmd.arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.output().map_err(mlua::Error::external)
}

// ========================= SHELL MODULE =========================
//
// shell.exec(command: string, admin?: boolean) -> { code, stdout, stderr }
// Always synchronous. For detached processes, use proc.spawn.
//

pub fn make_shell_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    let shell_module = lua.create_table()?;

    // Capture cwd as owned PathBuf so we can move it into the closure
    // and make sure it's absolute & canonicalized when possible.
    let cwd_buf: Option<PathBuf> = canonicalize_cwd(base_cwd);

    shell_module.set(
        "exec",
        lua.create_function(
            move |lua, (command, admin): (String, Option<bool>)| {
                let admin = admin.unwrap_or(false);
                let result_table = lua.create_table()?;

                let cwd_opt = cwd_buf.as_deref();

                // ---------- ALWAYS FOREGROUND / BLOCKING ----------
                let output: Result<Output> = {
                    #[cfg(windows)]
                    {
                        run_windows_command(&command, cwd_opt, admin)
                    }
                    #[cfg(unix)]
                    {
                        run_unix_command(&command, cwd_opt, admin)
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

                Ok(result_table)
            },
        )?,
    )?;

    Ok(shell_module)
}

// ========================= MODULE FACTORY =========================

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

    let proc_cwd: Option<PathBuf> = canonicalize_cwd(cwd);

    // ==================== PROC MODULE ====================
    //
    // proc.spawn(cmd: string) -> { pid }
    //   - Windows: CreateProcessW (no shell)
    //   - Unix: fork + execve/execvp (no shell)
    // proc.kill_pid(pid: number) -> boolean
    // proc.kill_name(name: string) -> boolean
    //

    proc_module.set(
        "spawn",
        lua.create_function({
            let proc_cwd = proc_cwd.clone();
            move |lua, command: String| {
                let result_table = lua.create_table()?;

                // Resolve ./foo, ../bar, scripts/run.sh, etc. against cwd
                let cwd_opt = proc_cwd.as_deref();
                let resolved = resolve_spawn_command(&command, cwd_opt);

                // ---------- WINDOWS IMPLEMENTATION ----------
                #[cfg(windows)]
                {
                    use windows::core::{PCWSTR, PWSTR};
                    use windows::Win32::Foundation::{CloseHandle, GetLastError};
                    use windows::Win32::System::Threading::{
                        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW, CREATE_NO_WINDOW,
                    };

                    // Windows requires a mutable, null-terminated UTF-16 buffer
                    let mut cmd_w: Vec<u16> = resolved
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();

                    let si: STARTUPINFOW = STARTUPINFOW::default();

                    let mut pi: PROCESS_INFORMATION = PROCESS_INFORMATION::default();

                    let ok = unsafe {
                        CreateProcessW(
                            PCWSTR::null(),     // lpApplicationName (let Windows parse cmd)
                            Some(PWSTR(cmd_w.as_mut_ptr())), // lpCommandLine (mutable buffer)
                            Some(std::ptr::null()),   // lpProcessAttributes
                            Some(std::ptr::null()),   // lpThreadAttributes
                            false,       // bInheritHandles
                            CREATE_NO_WINDOW, // dwCreationFlags
                            Some(std::ptr::null()),   // lpEnvironment
                            PCWSTR::null(),     // lpCurrentDirectory
                            &si,                // lpStartupInfo
                            &mut pi,            // lpProcessInformation
                        )
                    };

                    if ok.is_err() {
                        let err = unsafe { GetLastError().0 };
                        return Err(mlua::Error::external(format!(
                            "CreateProcessW failed with error code {err}"
                        )));
                    }

                    let pid = pi.dwProcessId;
                    unsafe {
                        CloseHandle(pi.hThread).map_err(mlua::Error::external)?;
                        CloseHandle(pi.hProcess).map_err(mlua::Error::external)?;
                    }

                    result_table.set("pid", pid)?;
                    Ok(result_table)
                }

                // ---------- UNIX (LINUX + MACOS) IMPLEMENTATION ----------
                #[cfg(unix)]
                {
                    use nix::unistd::{execve, execvp, fork, ForkResult};
                    use std::{ffi::{CStr, CString, NulError}, result::Result as StdResult};

                    // Split resolved command
                    let parts: Vec<&str> = resolved.split_whitespace().collect();
                    if parts.is_empty() {
                        return Err(mlua::Error::external("spawn: empty command string"));
                    }

                    // Convert to CStrings
                    let cstrings: Vec<CString> = parts
                        .iter()
                        .map(|s| CString::new(*s))
                        .collect::<StdResult<Vec<CString>, NulError>>()
                        .map_err(mlua::Error::external)?;

                    let prog = &cstrings[0];
                    let argv: Vec<&CStr> = cstrings.iter().map(|s| s.as_c_str()).collect();

                    // Decide: path-like → execve; bare name → execvp
                    let prog_str = prog.to_string_lossy();
                    let is_path_like =
                        prog_str.starts_with("./")
                        || prog_str.starts_with("../")
                        || prog_str.contains('/')
                        || prog_str.starts_with(".\\")
                        || prog_str.starts_with("..\\")
                        || prog_str.contains('\\');

                    match unsafe { fork() } {
                        Ok(ForkResult::Parent { child }) => {
                            let pid: i32 = child.as_raw();
                            result_table.set("pid", pid)?;
                            Ok(result_table)
                        }
                        Ok(ForkResult::Child) => {
                            if is_path_like {
                                // Build envp from current env (or a filtered one)
                                let env_cstrings: Vec<CString> = std::env::vars_os()
                                    .filter_map(|(k, v)| {
                                        let mut kv = k.into_string().ok()?;
                                        kv.push('=');
                                        kv.push_str(&v.into_string().ok()?);
                                        CString::new(kv).ok()
                                    })
                                    .collect();
                                let envp: Vec<&CStr> =
                                    env_cstrings.iter().map(|s| s.as_c_str()).collect();

                                match execve(prog, &argv, &envp) {
                                    Err(e) => {
                                        eprintln!("execve failed for '{}': {e}", resolved);
                                        std::process::exit(127);
                                    }
                                    Ok(_) => {}
                                }

                            } else {
                                match execvp(prog, &argv) {
                                    Err(e) => {
                                        eprintln!("execvp failed for '{}': {e}", resolved);
                                        std::process::exit(127);
                                    }
                                }
                            }
                            unreachable!("execve/execvp should not return on success");
                        }
                        Err(e) => Err(mlua::Error::external(format!("fork failed: {e}"))),
                    }
                }
            }
        })?,
    )?;

    // We now rely only on your platform helpers for kill by PID
    proc_module.set(
        "kill_pid",
        lua.create_function(|_, pid: u32| Ok(kill_process_by_pid(pid)))?,
    )?;

    // Name-based kill: same as before, just uses your helper
    proc_module.set(
        "kill_name",
        lua.create_function(|_, name: String| Ok(kill_process_by_name(&name)))?,
    )?;

    // list is still a stub for now
    proc_module.set("list", lua.create_table()?)?;

    // ==================== TIME MODULE ====================

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

    // ==================== JSON MODULE ====================

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

    // ==================== LOG MODULE ====================

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
