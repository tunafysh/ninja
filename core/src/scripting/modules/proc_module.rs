use super::shared::{canonicalize_cwd, resolve_spawn_command};
use crate::utils::{kill_process_by_name, kill_process_by_pid};
use log::{debug, error, info, warn};
use mlua::{Lua, Result, Table};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

pub(crate) fn make_proc_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
    debug!(
        "make_proc_module: base_cwd = {:?}",
        base_cwd.map(|p| p.display().to_string())
    );

    let proc_module = lua.create_table()?;
    let proc_cwd: Option<PathBuf> = canonicalize_cwd(base_cwd);

    proc_module.set(
        "spawn",
        lua.create_async_function({
            let proc_cwd = proc_cwd.clone();
            move |lua, args: mlua::Value| {
                let proc_cwd = proc_cwd.clone();
                async move {
                    let (command, custom_cwd): (String, Option<PathBuf>) = match args {
                        mlua::Value::String(s) => (s.to_str()?.to_string(), None),
                        mlua::Value::Table(t) => {
                            let cmd: String = t.get("command").or_else(|_| t.get(1))?;
                            let cwd: Option<PathBuf> = t.get("cwd").ok();
                            (cmd, cwd)
                        }
                        _ => {
                            return Err(mlua::Error::external(
                                "spawn requires string or table with 'command' field",
                            ))
                        }
                    };

                    #[cfg(unix)]
                    {
                        debug!(
                            "proc.spawn unix: command='{}', custom_cwd={:?}",
                            command, custom_cwd
                        );
                        let cwd_to_use = custom_cwd.as_deref().or(proc_cwd.as_deref());
                        let resolved = resolve_spawn_command(&command, cwd_to_use, None)?;

                        let mut cmd = tokio::process::Command::new("sh");
                        cmd.args(["-c", &resolved]);

                        if let Some(cwd) = cwd_to_use {
                            cmd.current_dir(cwd);
                        }

                        cmd.stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null());

                        unsafe {
                            cmd.pre_exec(|| {
                                libc::setsid();
                                Ok(())
                            });
                        }

                        let child = cmd.spawn().map_err(|e| {
                            error!("proc.spawn: failed to spawn '{}': {}", command, e);
                            mlua::Error::external(format!("spawn failed: {}", e))
                        })?;

                        let pid = child.id().unwrap_or(0);
                        debug!("proc.spawn: spawned detached process with pid={}", pid);

                        let result_table = lua.create_table()?;
                        result_table.set("pid", pid)?;
                        Ok(result_table)
                    }

                    #[cfg(windows)]
                    unsafe {
                        use std::iter::once;

                        use windows::{
                            Win32::System::Threading::{
                                CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, CreateProcessW,
                                DETACHED_PROCESS, PROCESS_INFORMATION, STARTUPINFOW,
                            },
                            core::{PCWSTR, PWSTR},
                        };

                        let cwd_to_use = custom_cwd.as_deref().or(proc_cwd.as_deref());

                        let command =
                            resolve_spawn_command(&command, cwd_to_use, Some(true))?;
                        debug!(
                            "proc.spawn windows: command='{}', custom_cwd={:?}",
                            command, custom_cwd
                        );
                        let mut si = STARTUPINFOW::default();
                        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

                        let mut pi = PROCESS_INFORMATION::default();

                        let mut wide_command: Vec<u16> =
                            command.encode_utf16().chain(once(0)).collect();

                        let wide_cwd = if let Some(cwd) = cwd_to_use {
                            let cwd: Vec<u16> = cwd
                                .to_string_lossy()
                                .encode_utf16()
                                .chain(once(0))
                                .collect();
                            PCWSTR(cwd.as_ptr())
                        } else {
                            PCWSTR::null()
                        };

                        CreateProcessW(
                            PCWSTR::null(),
                            Some(PWSTR(wide_command.as_mut_ptr())),
                            None,
                            None,
                            false,
                            CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | DETACHED_PROCESS,
                            None,
                            wide_cwd,
                            &si,
                            &mut pi,
                        )
                        .map_err(|e| {
                            mlua::Error::ExternalError(std::sync::Arc::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "Error code: {}, with message: {}",
                                    e.code(),
                                    e.message()
                                ),
                            )))
                        })?;

                        info!("proc.spawn: spawned detached process with pid={}", pi.dwProcessId);

                        let result_table = lua.create_table()?;
                        result_table.set("pid", pi.dwProcessId)?;
                        Ok(result_table)
                    }
                }
            }
        })?,
    )?;

    proc_module.set(
        "kill_pid",
        lua.create_function(|_, pid: u32| {
            debug!("proc.kill_pid: pid={}", pid);
            let result = kill_process_by_pid(pid);
            info!("proc.kill_pid: result={}", result);
            Ok(result)
        })?,
    )?;

    proc_module.set(
        "kill_name",
        lua.create_function(|_, name: String| {
            debug!("proc.kill_name: name='{}'", name);
            let result = kill_process_by_name(&name);
            debug!("proc.kill_name: result={}", result);
            Ok(result)
        })?,
    )?;

    proc_module.set(
        "exec",
        lua.create_async_function({
            let proc_cwd = proc_cwd.clone();
            move |lua, args: mlua::Value| {
                let proc_cwd = proc_cwd.clone();
                async move {
                    let (command, timeout_secs, custom_cwd): (String, Option<u64>, Option<PathBuf>) =
                        match args {
                            mlua::Value::String(s) => (s.to_str()?.to_string(), None, None),
                            mlua::Value::Table(t) => {
                                let cmd: String = t.get("command").or_else(|_| t.get(1))?;
                                let timeout: Option<u64> = t.get("timeout").ok();
                                let cwd: Option<PathBuf> = t.get("cwd").ok();
                                (cmd, timeout, cwd)
                            }
                            _ => {
                                return Err(mlua::Error::external(
                                    "exec requires string or table with 'command' field",
                                ))
                            }
                        };

                    debug!(
                        "proc.exec: command='{}', timeout={:?}, custom_cwd={:?}",
                        command, timeout_secs, custom_cwd
                    );

                    let cwd_to_use = custom_cwd.as_deref().or(proc_cwd.as_deref());
                    let use_backslash = cfg!(windows);
                    let resolved =
                        resolve_spawn_command(&command, cwd_to_use, Some(use_backslash))?;

                    let mut cmd = tokio::process::Command::new(if cfg!(windows) {
                        "cmd"
                    } else {
                        "sh"
                    });

                    if cfg!(windows) {
                        cmd.args(["/C", &resolved]);
                    } else {
                        cmd.args(["-c", &resolved]);
                    }

                    if let Some(cwd) = cwd_to_use {
                        cmd.current_dir(cwd);
                    }

                    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

                    let mut child = cmd.spawn().map_err(|e| {
                        error!("proc.exec: failed to spawn '{}': {}", resolved, e);
                        mlua::Error::external(e)
                    })?;

                    let status = if let Some(timeout_s) = timeout_secs {
                        let timeout = Duration::from_secs(timeout_s);
                        debug!("proc.exec: using timeout {:?}", timeout);
                        match tokio::time::timeout(timeout, child.wait()).await {
                            Ok(Ok(status)) => status,
                            Ok(Err(e)) => {
                                error!("proc.exec: wait failed for '{}': {}", resolved, e);
                                return Err(mlua::Error::external(e));
                            }
                            Err(_) => {
                                warn!("proc.exec: timeout for '{}', terminating", resolved);
                                let _ = child.kill().await;
                                return Err(mlua::Error::external("Process timeout"));
                            }
                        }
                    } else {
                        child.wait().await.map_err(|e| {
                            error!("proc.exec: wait failed for '{}': {}", resolved, e);
                            mlua::Error::external(e)
                        })?
                    };

                    let result = lua.create_table()?;
                    result.set("success", status.success())?;
                    result.set("exit_code", status.code().unwrap_or(-1))?;

                    debug!("proc.exec: completed '{}' with status: {:?}", resolved, status);

                    Ok(result)
                }
            }
        })?,
    )?;

    debug!("make_proc_module: done");
    Ok(proc_module)
}
