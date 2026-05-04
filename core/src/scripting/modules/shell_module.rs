use super::shared::canonicalize_cwd;
use log::{debug, error};
use mlua::{Lua, Result, Table};
use std::{
    path::Path,
    path::PathBuf,
    process::{Command, Stdio, Output},
};

struct ShellCommandResult {
    code: i32,
    stdout: String,
    stderr: String,
}

impl From<Output> for ShellCommandResult {
    fn from(output: Output) -> Self {
        ShellCommandResult {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }
}

#[cfg(windows)]
fn run_windows_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<ShellCommandResult> {
    use runas::Command as RunasCommand;
    debug!(
        "run_windows_command: command='{}', cwd={:?}, admin={}",
        command,
        cwd.map(|p| p.display().to_string()),
        admin
    );

    let mut cmd = if admin {
        RunasCommand::new("cmd")
    } else {
        Command::new("cmd")
    };

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    {
        cmd.arg("/C")
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

    Ok(ShellCommandResult::from(out))
}

#[cfg(target_os = "linux")]
fn make_admin_command(shell: &str) -> Option<Command> {
    use std::io::{self, IsTerminal};
    
    if !io::stdin().is_terminal() {
        debug!("run_unix_command: admin=true, using pkexec on Linux");
        let mut c = Command::new("pkexec");
        c.arg("--keep-cwd");
        c.arg(shell);
        Some(c)
    } else {
        debug!("run_unix_command: admin=true, using sudo on Linux");
        let mut c = Command::new("sudo");
        c.arg(shell);
        Some(c)
    }
}

#[cfg(target_os = "macos")]
fn make_admin_command_macos(shell: &str, cwd: Option<&Path>, command: &str) -> Result<ShellCommandResult> {
    use runas::Command as RunasCommand;
    
    debug!("run_unix_command: admin=true, executing with runas on macOS");
    let mut cmd = RunasCommand::new(shell);
    
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let out = cmd
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| {
            error!("run_unix_command: failed to execute admin command: {}", e);
            mlua::Error::external(e)
        })?;

    debug!(
        "run_unix_command: admin exit_code={:?}, stdout_len={}, stderr_len={}",
        out.status.code(),
        out.stdout.len(),
        out.stderr.len()
    );

    Ok(ShellCommandResult::from(out))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn make_admin_command(shell: &str) -> Option<Command> {
    debug!("run_unix_command: admin=true, using sudo");
    let mut c = Command::new("sudo");
    c.arg(shell);
    Some(c)
}

#[cfg(unix)]
fn run_unix_command(command: &str, cwd: Option<&Path>, admin: bool) -> Result<ShellCommandResult> {
    use std::env;
    
    debug!(
        "run_unix_command: command='{}', cwd={:?}, admin={}",
        command,
        cwd.map(|p| p.display().to_string()),
        admin
    );
    
    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    // Handle macOS admin execution with runas
    #[cfg(target_os = "macos")]
    if admin {
        return make_admin_command_macos(&shell, cwd, command);
    }

    // Standard execution path for non-admin or non-macOS
    let mut cmd = if admin {
        make_admin_command(&shell).unwrap_or_else(|| Command::new(&shell))
    } else {
        Command::new(&shell)
    };

    cmd.arg("-c")
        .arg(command)
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

    Ok(ShellCommandResult::from(out))
}

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
                    let code = cmd_output.code;
                    let stdout = cmd_output.stdout;
                    let stderr = cmd_output.stderr;

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
        })?,
    )?;

    debug!("make_shell_module: done");
    Ok(shell_module)
}
