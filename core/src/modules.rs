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

#[cfg(windows)]
fn strip_windows_prefix(p: &Path) -> PathBuf {
    debug!(
        "strip_windows_prefix (windows): original path: '{}'",
        p.display()
    );
    let s = p.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        let result = PathBuf::from(stripped);
        debug!(
            "strip_windows_prefix: stripped prefix, result: '{}'",
            result.display()
        );
        result
    } else {
        debug!(
            "strip_windows_prefix: no prefix to strip, returning original: '{}'",
            p.display()
        );
        p.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_windows_prefix(p: &Path) -> PathBuf {
    p.to_path_buf()
}

fn canonicalize_cwd(base: Option<&Path>) -> Option<PathBuf> {
    debug!(
        "canonicalize_cwd: base = {:?}",
        base.map(|p| p.display().to_string())
    );

    let result = base.map(|p| {
        let abs = if p.is_absolute() {
            debug!("canonicalize_cwd: base is absolute: '{}'", p.display());
            p.to_path_buf()
        } else {
            let cwd = env::current_dir().unwrap_or_else(|e| {
                warn!(
                    "canonicalize_cwd: failed to get current_dir ({}), using '.'",
                    e
                );
                PathBuf::from(".")
            });
            debug!(
                "canonicalize_cwd: base is relative '{}', joining with cwd '{}'",
                p.display(),
                cwd.display()
            );
            cwd.join(p)
        };

        let canon = abs.canonicalize().unwrap_or_else(|e| {
            warn!(
                "canonicalize_cwd: canonicalize failed for '{}', using abs as fallback: {}",
                abs.display(),
                e
            );
            abs
        });

        let stripped = strip_windows_prefix(&canon);
        debug!(
            "canonicalize_cwd: final result = '{}'",
            stripped.display()
        );
        stripped
    });

    debug!(
        "canonicalize_cwd: returning {:?}",
        result.as_ref().map(|p| p.display().to_string())
    );
    result
}

fn resolve_spawn_command(command: &str, cwd: Option<&Path>) -> String {
    debug!(
        "resolve_spawn_command: command='{}', cwd={:?}",
        command,
        cwd.map(|p| p.display().to_string())
    );

    let mut parts_iter = command.split_whitespace();
    let Some(first) = parts_iter.next() else {
        debug!("resolve_spawn_command: empty command, returning as-is");
        return command.to_string();
    };

    let rest: Vec<&str> = parts_iter.collect();

    let is_path_like = first.starts_with("./")
        || first.starts_with(".\\")
        || first.starts_with("../")
        || first.starts_with("..\\")
        || first.contains('/')
        || first.contains('\\');

    if !is_path_like {
        debug!(
            "resolve_spawn_command: '{}' is not path-like, using original command",
            first
        );
        return command.to_string();
    }

    let Some(cwd) = cwd else {
        debug!(
            "resolve_spawn_command: path-like but no cwd, using original command"
        );
        return command.to_string();
    };

    let first_path = Path::new(first);
    let abs = if first_path.is_absolute() {
        debug!(
            "resolve_spawn_command: '{}' is absolute path",
            first_path.display()
        );
        first_path.to_path_buf()
    } else {
        debug!(
            "resolve_spawn_command: '{}' is relative, resolving against '{}'",
            first_path.display(),
            cwd.display()
        );
        let rel = RelativePath::new(first);
        rel.to_logical_path(cwd)
    };

    let abs = strip_windows_prefix(&abs);
    let mut cmd = abs.to_string_lossy().to_string();
    if !rest.is_empty() {
        cmd.push(' ');
        cmd.push_str(&rest.join(" "));
    }

    debug!(
        "resolve_spawn_command: resolved command='{}'",
        cmd
    );
    cmd
}


// ========================= FS MODULE =========================

pub fn make_fs_module(lua: &Lua, cwd: Option<&Path>) -> Result<Table> {
    debug!(
        "make_fs_module: cwd = {:?}",
        cwd.map(|p| p.display().to_string())
    );
    let fs_module = lua.create_table()?;

    let base_cwd: Option<PathBuf> = cwd.map(|p| p.to_path_buf());
    debug!(
        "make_fs_module: base_cwd = {:?}",
        base_cwd.as_ref().map(|p| p.display().to_string())
    );

    fn resolve_with_cwd(base_cwd: &Option<PathBuf>, path: &PathBuf) -> PathBuf {
        if let Some(cwd) = base_cwd {
            let resolved = resolve_path(cwd, path);
            debug!(
                "fs: resolved '{}' against '{}' -> '{}'",
                path.display(),
                cwd.display(),
                resolved.display()
            );
            resolved
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
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.read: '{}'", resolved.display());
                fs::read_to_string(&resolved).map_err(|e| {
                    error!("fs.read: failed for '{}': {}", resolved.display(), e);
                    mlua::Error::external(e)
                })
            })?,
        )?;
    }

    // fs.write(path, content)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "write",
            lua.create_function(move |_, (path, content): (PathBuf, String)| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!(
                    "fs.write: '{}' (len={})",
                    resolved.display(),
                    content.len()
                );
                fs::write(&resolved, content).map_err(|e| {
                    error!("fs.write: failed for '{}': {}", resolved.display(), e);
                    mlua::Error::external(e)
                })
            })?,
        )?;
    }

    // fs.append(path, content)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "append",
            lua.create_function(move |_, (path, content): (PathBuf, String)| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!(
                    "fs.append: '{}' (len={})",
                    resolved.display(),
                    content.len()
                );
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&resolved)
                    .map_err(|e| {
                        error!(
                            "fs.append: failed to open '{}': {}",
                            resolved.display(),
                            e
                        );
                        mlua::Error::external(e)
                    })?;
                file.write_all(content.as_bytes()).map_err(|e| {
                    error!(
                        "fs.append: failed to write to '{}': {}",
                        resolved.display(),
                        e
                    );
                    mlua::Error::external(e)
                })?;
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
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.remove: '{}'", resolved.display());
                fs::remove_file(&resolved).map_err(|e| {
                    error!(
                        "fs.remove: failed for '{}': {}",
                        resolved.display(),
                        e
                    );
                    mlua::Error::external(e)
                })?;
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
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.create_dir: '{}'", resolved.display());
                fs::create_dir_all(&resolved).map_err(|e| {
                    error!(
                        "fs.create_dir: failed for '{}': {}",
                        resolved.display(),
                        e
                    );
                    mlua::Error::external(e)
                })?;
                Ok(())
            })?,
        )?;
    }

    // fs.read_dir(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "read_dir",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.read_dir: '{}'", resolved.display());
                let entries = fs::read_dir(&resolved).map_err(|e| {
                    error!(
                        "fs.read_dir: failed for '{}': {}",
                        resolved.display(),
                        e
                    );
                    mlua::Error::external(e)
                })?;

                let mut result = Vec::new();
                for entry in entries.flatten() {
                    let name_os = entry.file_name();
                    match name_os.into_string() {
                        Ok(name) => result.push(name),
                        Err(_) => {
                            warn!(
                                "fs.read_dir: invalid UTF-8 entry in '{}'",
                                resolved.display()
                            );
                            result.push("<invalid UTF-8>".to_string());
                        }
                    }
                }

                debug!(
                    "fs.read_dir: '{}' -> {} entries",
                    resolved.display(),
                    result.len()
                );
                Ok(result)
            })?,
        )?;
    }

    // fs.exists(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "exists",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                let exists = resolved.exists();
                debug!("fs.exists: '{}' -> {}", resolved.display(), exists);
                Ok(exists)
            })?,
        )?;
    }

    // fs.is_dir(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "is_dir",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                let is_dir = resolved.is_dir();
                debug!("fs.is_dir: '{}' -> {}", resolved.display(), is_dir);
                Ok(is_dir)
            })?,
        )?;
    }

    // fs.is_file(path)
    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "is_file",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                let is_file = resolved.is_file();
                debug!("fs.is_file: '{}' -> {}", resolved.display(), is_file);
                Ok(is_file)
            })?,
        )?;
    }

    debug!("make_fs_module: done");
    Ok(fs_module)
}

// ========================= ENV MODULE =========================

pub fn make_env_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    debug!(
        "make_env_module: base_cwd = {:?}",
        base_cwd.map(|p| p.display().to_string())
    );
    let env_module = lua.create_table()?;

    env_module.set("os", env::consts::OS)?;
    env_module.set("arch", env::consts::ARCH)?;

    env_module.set(
        "get",
        lua.create_function(|_, key: String| {
            let val = env::var(&key).ok();
            debug!("env.get: key='{}' -> {:?}", key, val);
            Ok(val)
        })?,
    )?;
    env_module.set(
        "set",
        lua.create_function(|_, (key, value): (String, String)| unsafe {
            debug!("env.set: key='{}'", key);
            env::set_var(&key, &value);
            Ok(())
        })?,
    )?;
    env_module.set(
        "remove",
        lua.create_function(|_, key: String| unsafe {
            debug!("env.remove: key='{}'", key);
            env::remove_var(&key);
            Ok(())
        })?,
    )?;
    env_module.set(
        "vars",
        lua.create_function(|lua, _: ()| {
            debug!("env.vars: listing env vars");
            let table = lua.create_table()?;
            for (k, v) in env::vars() {
                table.set(k, v)?;
            }
            Ok(table)
        })?,
    )?;

    let cwd_string = canonicalize_cwd(base_cwd)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    debug!("make_env_module: cwd='{}'", cwd_string);

    env_module.set(
        "cwd",
        lua.create_function(move |_, _: ()| Ok(cwd_string.clone()))?,
    )?;

    debug!("make_env_module: done");
    Ok(env_module)
}

// ========================= SHELL HELPERS =========================

#[cfg(windows)]
fn run_windows_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<Output> {
    debug!(
        "run_windows_command: command='{}', cwd={:?}, admin={}",
        command,
        cwd.map(|p| p.display().to_string()),
        admin
    );

    let mut cmd = Command::new("powershell.exe");
    cmd.arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden");

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    if admin {
        // NOTE: This is not a real -Verb RunAs usage on powershell.exe, but preserved
        debug!("run_windows_command: admin=true (RunAs-like)");
        cmd.arg("-Verb").arg("RunAs");
    }

    {
        cmd.arg("-Command")
           .arg(command)
           .stdin(Stdio::inherit())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
    }
    let out = cmd.output().map_err(|e| {
        error!("run_windows_command: failed to execute: {}", e);
        mlua::Error::external(e)
    })?;

    debug!(
        "run_windows_command: status={:?}, stdout_len={}, stderr_len={}",
        out.status.code(),
        out.stdout.len(),
        out.stderr.len()
    );

    Ok(out)
}

#[cfg(unix)]
fn run_unix_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<Output> {
    debug!(
        "run_unix_command: command='{}', cwd={:?}, admin={}",
        command,
        cwd.map(|p| p.display().to_string()),
        admin
    );
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    let mut cmd = if admin {
        debug!("run_unix_command: admin=true, using pkexec");
        let mut c = Command::new("pkexec");
        c.arg("--keep-cwd");
        c.arg(&shell);
        c
    } else {
        Command::new(&shell)
    };

    cmd.arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let out = cmd.output().map_err(|e| {
        error!("run_unix_command: failed to execute: {}", e);
        mlua::Error::external(e)
    })?;

    debug!(
        "run_unix_command: status={:?}, stdout_len={}, stderr_len={}",
        out.status.code(),
        out.stdout.len(),
        out.stderr.len()
    );

    Ok(out)
}

// ========================= SHELL MODULE =========================

pub fn make_shell_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    debug!(
        "make_shell_module: base_cwd = {:?}",
        base_cwd.map(|p| p.display().to_string())
    );
    let shell_module = lua.create_table()?;

    let cwd_buf: Option<PathBuf> = canonicalize_cwd(base_cwd);
    debug!(
        "make_shell_module: cwd_buf = {:?}",
        cwd_buf.as_ref().map(|p| p.display().to_string())
    );

    shell_module.set(
        "exec",
        lua.create_function(
            move |lua, (command, admin): (String, Option<bool>)| {
                let admin = admin.unwrap_or(false);
                debug!(
                    "shell.exec: command='{}', admin={}, cwd={:?}",
                    command,
                    admin,
                    cwd_buf.as_ref().map(|p| p.display().to_string())
                );
                let result_table = lua.create_table()?;

                let cwd_opt = cwd_buf.as_deref();

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
                        let code = cmd_output.status.code().unwrap_or(-1);
                        let stdout = String::from_utf8_lossy(&cmd_output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&cmd_output.stderr).to_string();

                        debug!(
                            "shell.exec: exit_code={}, stdout_len={}, stderr_len={}",
                            code,
                            stdout.len(),
                            stderr.len()
                        );

                        result_table.set("code", code)?;
                        result_table.set("stdout", stdout)?;
                        result_table.set("stderr", stderr)?;
                    }
                    Err(e) => {
                        error!("shell.exec: failed to execute '{}': {}", command, e);
                        result_table.set("code", -1)?;
                        result_table.set("stdout", "")?;
                        result_table.set("stderr", format!("Failed: {}", e))?;
                    }
                }

                Ok(result_table)
            },
        )?,
    )?;

    debug!("make_shell_module: done");
    Ok(shell_module)
}

#[cfg(windows)]
fn format_win32_error(code: u32) -> String {
    use windows::Win32::System::Diagnostics::Debug::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS};
    use windows::core::PWSTR;

    let mut buf: [u16; 512] = [0; 512];

    unsafe {
        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            None,
            code,
            0,
            PWSTR(buf.as_mut_ptr()),
            buf.len() as u32,
            None,
        );

        if len == 0 {
            return format!("Unknown Win32 error {}", code);
        }

        let s = String::from_utf16_lossy(&buf[..len as usize]);
        s.trim().to_string()
    }
}

// ========================= MODULE FACTORY =========================

pub async fn make_modules(
    lua: &Lua,
    cwd: Option<&Path>,
) -> Result<(Table, Table, Table, Table, Table, Table, Table, Table)> {
    debug!(
        "make_modules: cwd = {:?}",
        cwd.map(|p| p.display().to_string())
    );

    let fs_module = make_fs_module(lua, cwd)?;
    let env_module = make_env_module(lua, cwd)?;
    let shell_module = make_shell_module(lua, cwd)?;
    let time_module = lua.create_table()?;
    let json_module = lua.create_table()?;
    let http_module = lua.create_table()?;
    let log_module = lua.create_table()?;
    let proc_module = lua.create_table()?;

    let proc_cwd: Option<PathBuf> = canonicalize_cwd(cwd);
    debug!(
        "make_modules: proc_cwd = {:?}",
        proc_cwd.as_ref().map(|p| p.display().to_string())
    );

    // ==================== PROC MODULE ====================

    proc_module.set(
        "spawn",
        lua.create_function({
            let proc_cwd = proc_cwd.clone();
            move |lua, command: String| {
                debug!(
                    "proc.spawn: command='{}', proc_cwd={:?}",
                    command,
                    proc_cwd.as_ref().map(|p| p.display().to_string())
                );
                let result_table = lua.create_table()?;

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

                    debug!(
                        "proc.spawn (windows): original='{}', resolved='{}'",
                        command, resolved
                    );

                    let mut cmd_w: Vec<u16> = resolved
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();

                    let mut si: STARTUPINFOW = STARTUPINFOW::default();
                    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

                    let mut pi: PROCESS_INFORMATION = PROCESS_INFORMATION::default();

                    let ok = unsafe {
                        CreateProcessW(
                            PCWSTR::null(),                   // lpApplicationName
                            Some(PWSTR(cmd_w.as_mut_ptr())), // lpCommandLine
                            None,
                            None,
                            false,
                            CREATE_NO_WINDOW,
                            None,
                            PCWSTR::null(), // lpCurrentDirectory (inherits from parent)
                            &si,
                            &mut pi,
                        )
                    };

                    if let Err(e) = ok {
                        let code = unsafe { GetLastError().0 };
                        let msg = format_win32_error(code);
                        error!(
                            "proc.spawn (windows): CreateProcessW failed\n  original='{}'\n  resolved='{}'\n  code={}\n  message='{}'\n  api_error='{}'",
                            command,
                            resolved,
                            code,
                            msg,
                            e
                        );
                        return Err(mlua::Error::external(format!(
                            "CreateProcessW failed for '{}': code {} ({})",
                            resolved, code, msg
                        )));
                    }

                    let pid = pi.dwProcessId;
                    debug!("proc.spawn (windows): spawned pid={}", pid);

                    unsafe {
                        if let Err(e) = CloseHandle(pi.hThread) {
                            warn!(
                                "proc.spawn (windows): CloseHandle(hThread) failed for pid {}: {}",
                                pid, e
                            );
                        }
                        if let Err(e) = CloseHandle(pi.hProcess) {
                            warn!(
                                "proc.spawn (windows): CloseHandle(hProcess) failed for pid {}: {}",
                                pid, e
                            );
                        }
                    }

                    result_table.set("pid", pid)?;
                    return Ok(result_table);
                }

                // ---------- UNIX (LINUX + MACOS) IMPLEMENTATION ----------
                #[cfg(unix)]
                {
                    use nix::unistd::{execve, execvp, fork, ForkResult};
                    use std::{
                        ffi::{CStr, CString, NulError},
                        result::Result as StdResult,
                    };

                    debug!(
                        "proc.spawn (unix): original='{}', resolved='{}'",
                        command, resolved
                    );

                    let parts: Vec<&str> = resolved.split_whitespace().collect();
                    if parts.is_empty() {
                        error!("proc.spawn (unix): empty command after resolve");
                        return Err(mlua::Error::external("spawn: empty command string"));
                    }

                    let cstrings: Vec<CString> = parts
                        .iter()
                        .map(|s| CString::new(*s))
                        .collect::<StdResult<Vec<CString>, NulError>>()
                        .map_err(|e| {
                            error!(
                                "proc.spawn (unix): failed to build argv for '{}': {}",
                                resolved, e
                            );
                            mlua::Error::external(e)
                        })?;

                    let prog = &cstrings[0];
                    let argv: Vec<&CStr> = cstrings.iter().map(|s| s.as_c_str()).collect();

                    let prog_str = prog.to_string_lossy();
                    let is_path_like = prog_str.starts_with("./")
                        || prog_str.starts_with("../")
                        || prog_str.contains('/')
                        || prog_str.starts_with(".\\")
                        || prog_str.starts_with("..\\")
                        || prog_str.contains('\\');

                    debug!(
                        "proc.spawn (unix): prog='{}', is_path_like={}",
                        prog_str, is_path_like
                    );

                    match unsafe { fork() } {
                        Ok(ForkResult::Parent { child }) => {
                            let pid: i32 = child.as_raw();
                            debug!("proc.spawn (unix): parent, child pid={}", pid);
                            result_table.set("pid", pid)?;
                            Ok(result_table)
                        }
                        Ok(ForkResult::Child) => {
                            debug!(
                                "proc.spawn (unix): child, using {}",
                                if is_path_like { "execve" } else { "execvp" }
                            );

                            if is_path_like {
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
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("execve failed for '{}': {e}", resolved);
                                        error!(
                                            "proc.spawn (unix): execve failed for '{}': {}",
                                            resolved, e
                                        );
                                        std::process::exit(127);
                                    }
                                }
                            } else 
                            {
                                match execvp(prog, &argv){
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("execvp failed for '{}': {e}", resolved);
                                        error!(
                                            "proc.spawn (unix): execvp failed for '{}': {}",
                                            resolved, e
                                        );
                                        std::process::exit(127);
                                    }
                                }
                            }
                            unreachable!("execve/execvp should not return on success");
                        }
                        Err(e) => {
                            error!("proc.spawn (unix): fork failed: {}", e);
                            Err(mlua::Error::external(format!("fork failed: {e}")))
                        }
                    }
                }
            }
        })?,
    )?;

    proc_module.set(
        "kill_pid",
        lua.create_function(|_, pid: u32| {
            #[cfg(windows)]
            {
                debug!("proc.kill_pid: pid={}", pid);
                let result = kill_process_by_pid(pid)?;
                debug!("proc.kill_pid: result={} for pid={}", result, pid);
                Ok(result)
            }
            
            #[cfg(not(windows))]
            {
                debug!("proc.kill_pid: pid={}", pid);
                let result = kill_process_by_pid(pid);
                debug!("proc.kill_pid: result={} for pid={}", result, pid);
                Ok(result)
            }
        })?,
    )?;

    proc_module.set(
        "kill_name",
        lua.create_function(|_, name: String| {
            #[cfg(windows)]
            {
                debug!("proc.kill_name: name='{}'", name);
                let result = kill_process_by_name(&name)?;
                debug!("proc.kill_name: result={} for name='{}'", result, name);
                Ok(result)
            }
            
            #[cfg(not(windows))]
            {
                debug!("proc.kill_name: name='{}'", name);
                let result = kill_process_by_name(&name);
                debug!("proc.kill_name: result={} for name='{}'", result, name);
                Ok(result)
            }
        })?,
    )?;

    proc_module.set("list", lua.create_table()?)?;

    // ==================== TIME MODULE ====================

    time_module.set(
        "year",
        lua.create_function(|_, _: ()| {
            let y = Utc::now().year();
            debug!("time.year -> {}", y);
            Ok(y)
        })?,
    )?;
    time_module.set(
        "month",
        lua.create_function(|_, _: ()| {
            let m = Utc::now().month();
            debug!("time.month -> {}", m);
            Ok(m)
        })?,
    )?;
    time_module.set(
        "day",
        lua.create_function(|_, _: ()| {
            let d = Utc::now().day();
            debug!("time.day -> {}", d);
            Ok(d)
        })?,
    )?;
    time_module.set(
        "hour",
        lua.create_function(|_, format: bool| {
            let now = Utc::now();
            if format {
                let (pm, hour) = now.hour12();
                debug!(
                    "time.hour(format=true) -> {} {}",
                    hour,
                    if pm { "PM" } else { "AM" }
                );
                Ok((hour, if pm { "PM" } else { "AM" }))
            } else {
                let h = now.hour();
                debug!("time.hour(format=false) -> {}", h);
                Ok((h, ""))
            }
        })?,
    )?;
    time_module.set(
        "minute",
        lua.create_function(|_, _: ()| {
            let m = Utc::now().minute();
            debug!("time.minute -> {}", m);
            Ok(m)
        })?,
    )?;
    time_module.set(
        "second",
        lua.create_function(|_, _: ()| {
            let s = Utc::now().second();
            debug!("time.second -> {}", s);
            Ok(s)
        })?,
    )?;
    time_module.set(
        "now",
        lua.create_function(|_, fmt: String| {
            let now = Utc::now().format(&fmt).to_string();
            debug!("time.now('{}') -> '{}'", fmt, now);
            Ok(now)
        })?,
    )?;
    time_module.set(
        "sleep",
        lua.create_function(|_, seconds: f64| {
            debug!("time.sleep: {} seconds", seconds);
            std::thread::sleep(Duration::from_secs_f64(seconds));
            Ok(())
        })?,
    )?;

    // ==================== JSON MODULE ====================

    json_module.set(
        "encode",
        lua.create_function(|_, table: Table| {
            debug!("json.encode");
            serde_json::to_string(&table).map_err(|e| {
                error!("json.encode: failed: {}", e);
                e.into_lua_err()
            })
        })?,
    )?;
    json_module.set(
        "decode",
        lua.create_function(|lua, s: String| {
            debug!("json.decode: len={}", s.len());
            let val: Value = match serde_json::from_str(&s) {
                Ok(v) => v,
                Err(e) => {
                    error!("json.decode: invalid JSON: {} | '{}'", e, s);
                    Value::String(format!("Invalid JSON: {}", s))
                }
            };
            match lua.to_value(&val) {
                Ok(mlua::Value::Table(t)) => Ok(t),
                _ => {
                    error!("json.decode: expected object/array");
                    Err(mlua::Error::DeserializeError(
                        "Expected JSON object/array".into(),
                    ))
                }
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

    debug!("make_modules: all modules created");
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
