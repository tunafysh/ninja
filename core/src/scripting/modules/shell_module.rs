use super::shared::canonicalize_cwd;
use log::{debug, error};
use mlua::{Lua, Result, Table};
use std::{path::Path, path::PathBuf, process::Command};

struct ShellCommandResult {
    code: i32,
}

// Helper: Build command with cd prepended
fn build_command_with_cwd(command: &str, cwd: Option<&Path>) -> String {
    if let Some(cwd) = cwd {
        match cfg!(target_os = "windows") {
            true => format!("cd /d \"{}\" && {}", cwd.display(), command),
            false => format!("cd '{}' && {}", cwd.display(), command),
        }
    } else {
        command.to_string()
    }
}

// ============================================================================
// WINDOWS
// ============================================================================

#[cfg(windows)]
fn run_windows_admin(command: &str, cwd: Option<&Path>) -> Result<ShellCommandResult> {
    use runas::Command as RunasCommand;

    let full_cmd = build_command_with_cwd(command, cwd);
    debug!("run_windows_admin: {}", full_cmd);

    let status = RunasCommand::new("cmd")
        .arg("/C")
        .arg(&full_cmd)
        .status()
        .map_err(|e| {
            error!("run_windows_admin: {}", e);
            mlua::Error::external(e)
        })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_windows_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(windows)]
fn run_windows_non_admin(command: &str, cwd: Option<&Path>) -> Result<ShellCommandResult> {
    let mut cmd = Command::new("cmd");

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.arg("/C").arg(command);

    debug!("run_windows_non_admin: {}", command);

    let status = cmd.status().map_err(|e| {
        error!("run_windows_non_admin: {}", e);
        mlua::Error::external(e)
    })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_windows_non_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(windows)]
fn run_windows_command(
    command: &str,
    cwd: Option<&Path>,
    admin: bool,
) -> Result<ShellCommandResult> {
    if admin {
        run_windows_admin(command, cwd)
    } else {
        run_windows_non_admin(command, cwd)
    }
}

// ============================================================================
// UNIX/MACOS
// ============================================================================

#[cfg(target_os = "macos")]
fn run_unix_admin(command: &str, cwd: Option<&Path>, shell: &str) -> Result<ShellCommandResult> {
    use runas::Command as RunasCommand;
    let full_cmd = build_command_with_cwd(command, cwd);

    debug!("run_unix_admin (macOS): {}", full_cmd);

    let status = RunasCommand::new(shell)
        .arg("-c")
        .arg(&full_cmd)
        .status()
        .map_err(|e| {
            error!("run_unix_admin: {}", e);
            mlua::Error::external(e)
        })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_unix_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(target_os = "linux")]
fn run_unix_admin(command: &str, cwd: Option<&Path>, shell: &str) -> Result<ShellCommandResult> {
    use std::io::{self, IsTerminal};
    let full_cmd = build_command_with_cwd(command, cwd);

    let mut cmd = if !io::stdin().is_terminal() {
        debug!("run_unix_admin (Linux non-interactive): using pkexec");
        let mut c = Command::new("pkexec");
        c.arg("--keep-cwd");
        c
    } else {
        debug!("run_unix_admin (Linux interactive): using sudo");
        Command::new("sudo")
    };

    cmd.arg(shell).arg("-c").arg(&full_cmd);

    debug!("run_unix_admin: {}", full_cmd);

    let status = cmd.status().map_err(|e| {
        error!("run_unix_admin: {}", e);
        mlua::Error::external(e)
    })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_unix_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn run_unix_admin(command: &str, cwd: Option<&Path>, shell: &str) -> Result<ShellCommandResult> {
    let full_cmd = build_command_with_cwd(command, cwd);

    debug!("run_unix_admin: using sudo");

    let status = Command::new("sudo")
        .arg(shell)
        .arg("-c")
        .arg(&full_cmd)
        .status()
        .map_err(|e| {
            error!("run_unix_admin: {}", e);
            mlua::Error::external(e)
        })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_unix_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(unix)]
fn run_unix_non_admin(
    command: &str,
    cwd: Option<&Path>,
    shell: &str,
) -> Result<ShellCommandResult> {
    let mut cmd = Command::new(shell);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    cmd.arg("-c").arg(command);

    debug!("run_unix_non_admin: {}", command);

    let status = cmd.status().map_err(|e| {
        error!("run_unix_non_admin: {}", e);
        mlua::Error::external(e)
    })?;

    let code = status.code().unwrap_or(-1);
    debug!("run_unix_non_admin: exit={}", code);
    Ok(ShellCommandResult { code })
}

#[cfg(unix)]
fn run_unix_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<ShellCommandResult> {
    use std::env;
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    debug!(
        "run_unix_command: command='{}', admin={}, cwd={:?}",
        command,
        admin,
        cwd.map(|p| p.display().to_string())
    );

    if admin {
        run_unix_admin(command, cwd, &shell)
    } else {
        run_unix_non_admin(command, cwd, &shell)
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

pub(crate) fn make_shell_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
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
        lua.create_function(move |lua, (command, admin): (String, Option<bool>)| {
            let admin = admin.unwrap_or(false);
            debug!(
                "shell.exec: command='{}', admin={}, cwd={:?}",
                command,
                admin,
                cwd_buf.as_ref().map(|p| p.display().to_string())
            );
            let result_table = lua.create_table()?;

            let cwd_opt = cwd_buf.as_deref();

            let output: Result<ShellCommandResult> = {
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
                    debug!("shell.exec: exit_code={}", cmd_output.code);
                    result_table.set("code", cmd_output.code)?;
                }
                Err(e) => {
                    error!("shell.exec: failed to execute '{}': {}", command, e);
                    result_table.set("code", -1)?;
                }
            }

            Ok(result_table)
        })?,
    )?;

    debug!("make_shell_module: done");
    Ok(shell_module)
}
