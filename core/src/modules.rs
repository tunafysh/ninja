use chrono::prelude::*;
use log::{debug, error, info, warn};
use mlua::{ExternalError, Lua, LuaSerdeExt, Result, Table};
use serde_json::Value;
//use crate::manager::ShurikenManager;
use std::{
    env, fs, io,
    io::Write,
    path::Path,
    process::{Command, Output},
    time::Duration,
};

#[cfg(windows)]
fn run_windows_command(command: &str) -> Result<std::process::Output> {
    use std::process::Stdio;

    match Command::new("cmd")
        .arg("/C")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => return Ok(output),
        Err(_) => Err(mlua::Error::external(io::Error::new(
            io::ErrorKind::NotFound,
            "No shell found",
        ))),
    }

    // If all failed, return the last error
}

#[cfg(unix)]
fn run_unix_command(command: &str) -> Result<std::process::Output> {
    use std::process::Stdio;

    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    match Command::new(shell)
        .args(["-c", command])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => Ok(output),
        Err(e) => Err(mlua::Error::external(e)),
    }
}

pub fn make_modules(lua: &Lua) -> Result<(Table, Table, Table, Table, Table, Table, Table, Table)> {
    let ninja_module = lua.create_table()?;
    let fs_module = lua.create_table()?;
    let env_module = lua.create_table()?;
    let shell_module = lua.create_table()?;
    let time_module = lua.create_table()?;
    let json_module = lua.create_table()?;
    let http_module = lua.create_table()?;
    let log_module = lua.create_table()?;

    // =========== ninja module bootstrap ==========

    // let manager = ShurikenManager::new().await?;

    // ================= ninja module ==============

    // ninja_module.set(
    //     "start",
    //     lua.create_async_function(|_, name: String| async move {
    //         Ok(manager.start(name.as_str()).await.map_err(|e| LuaError::external(e))?)
    //     })?,
    // )?;

    // ================= fs module =================

    fs_module.set(
        "read",
        lua.create_function(|_, path: String| Ok(fs::read_to_string(path)?))?,
    )?;

    fs_module.set(
        "write",
        lua.create_function(|_, (path, content): (String, String)| Ok(fs::write(path, content)?))?,
    )?;

    fs_module.set(
        "append",
        lua.create_function(|_, (path, content): (String, String)| {
            let mut file = fs::OpenOptions::new().append(true).open(path)?;

            Ok(file.write_all(content.as_bytes())?)
        })?,
    )?;

    fs_module.set(
        "remove",
        lua.create_function(|_, path: String| Ok(fs::remove_file(path)?))?,
    )?;

    fs_module.set(
        "create_dir",
        lua.create_function(|_, path: String| Ok(fs::create_dir(path)?))?,
    )?;

    fs_module.set(
        "read_dir",
        lua.create_function(|_, path: String| {
            let entries: std::io::Result<Vec<String>> = std::fs::read_dir(path)?
                .map(|entry| {
                    entry.map(|e| {
                        e.file_name()
                            .into_string()
                            .unwrap_or_else(|os_str| format!("<invalid UTF-8: {:?}>", os_str))
                    })
                })
                .collect();

            match entries {
                Ok(files) => Ok(files),
                Err(e) => Err(mlua::Error::external(e)),
            }
        })?,
    )?;

    fs_module.set(
        "exists",
        lua.create_function(|_, path: String| Ok(fs::exists(path)?))?,
    )?;

    fs_module.set(
        "is_dir",
        lua.create_function(|_, path: String| Ok(Path::new(&path).is_dir()))?,
    )?;

    fs_module.set(
        "is_file",
        lua.create_function(|_, path: String| Ok(Path::new(&path).is_file()))?,
    )?;

    // ================= env module =================

    env_module.set("os", env::consts::OS)?;
    env_module.set("arch", env::consts::ARCH)?;

    env_module.set(
        "get",
        lua.create_function(|_, key: String| match env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(e) => {
                eprintln!("Failed to get environment variable, reason: {}", e);
                Ok(None)
            }
        })?,
    )?;

    env_module.set(
        "set",
        lua.create_function(|_, (key, value): (String, String)| unsafe {
            let _: () = env::set_var(key, value);
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
                table.set(k, v)?
            }
            Ok(table)
        })?,
    )?;

    env_module.set(
        "cwd",
        lua.create_function(|_, _: ()| Ok(env::current_dir()?))?,
    )?;

    // ================= shell module =================

    shell_module.set(
        "exec",
        lua.create_function(|lua, command: String| {
            // Create result table
            let result_table = lua.create_table()?;

            // Run command and capture output
            #[cfg(windows)]
            let output: Result<Output> = run_windows_command(&command);

            #[cfg(unix)]
            let output: Result<Output> = run_unix_command(&command);

            #[cfg(not(any(windows, unix)))]
            let output: Result<Output> = Err(mlua::Error::external("Unsupported OS"));

            match output {
                Ok(cmd_output) => {
                    let exit_code = cmd_output.status.code().unwrap_or(-1);
                    result_table.set("code", exit_code)?;
                    let stdout = String::from_utf8_lossy(&cmd_output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&cmd_output.stderr).to_string();
                    result_table.set("stdout", stdout)?;
                    result_table.set("stderr", stderr)?;
                }
                Err(e) => {
                    result_table.set("code", -1)?;
                    result_table.set("stdout", "")?;
                    result_table.set("stderr", format!("Command execution failed: {}", e))?;
                }
            }

            Ok(result_table)
        })?,
    )?;

    // ================= time module =================

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
                // Return 12-hour clock hour (1-12)
                let (is_pm, hour) = now.hour12();
                Ok((hour, if is_pm { "PM" } else { "AM" }))
            } else {
                // Return 24-hour clock hour (0-23)
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
        lua.create_function(|_, format: String| {
            Ok(Utc::now().format(format.as_str()).to_string())
        })?,
    )?;

    time_module.set(
        "sleep",
        lua.create_function(|_, seconds: f64| {
            let dur = Duration::from_secs_f64(seconds);
            std::thread::sleep(dur);
            Ok(())
        })?,
    )?;

    // ================= json module =================

    json_module.set(
        "encode",
        lua.create_function(|_, data: Table| match serde_json::to_string(&data) {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_lua_err()),
        })?,
    )?;

    json_module.set(
        "decode",
        lua.create_function(|lua, json_string: String| {
            let json_value: Value = match serde_json::from_str(&json_string) {
                Ok(val) => val,
                Err(e) => format!("Invalid JSON: {}", e).into(),
            };

            let lua_value = lua.to_value(&json_value)?;

            if let mlua::Value::Table(table) = lua_value {
                Ok(table)
            } else {
                Err(mlua::Error::DeserializeError(
                    "JSON did not represent an object or array — expected a table".to_string(),
                ))
            }
        })?,
    )?;

    // ================= http module =================

    //TODO find a reqwest substitute bc bro this shi sucks (actually just make the function async.
    // http_module.set("fetch", lua.create_function(|_, (url, method, headers): (String, Option<String>, Option<Table>)| {
    //     make_request(url, method, headers)
    // })?)?;

    // ================= log module =================

    log_module.set(
        "info",
        lua.create_function(|_, content: String| {
            info!("{}", content);
            Ok(())
        })?,
    )?;

    log_module.set(
        "warn",
        lua.create_function(|_, content: String| {
            warn!("{}", content);
            Ok(())
        })?,
    )?;

    log_module.set(
        "error",
        lua.create_function(|_, content: String| {
            error!("{}", content);
            Ok(())
        })?,
    )?;

    log_module.set(
        "debug",
        lua.create_function(|_, content: String| {
            debug!("{}", content);
            Ok(())
        })?,
    )?;

    Ok((
        ninja_module,
        fs_module,
        env_module,
        shell_module,
        time_module,
        json_module,
        http_module,
        log_module,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    async fn setup_lua() -> (Lua, (Table, Table, Table, Table, Table, Table, Table, Table)) {
        let lua = Lua::new();
        let modules = make_modules(&lua).expect("failed to create modules");
        (lua, modules)
    }

    #[tokio::test]
    async fn test_fs_write_and_read() {
        let (_lua, (_, fs_module, _, _, _, _, _, _)) = setup_lua().await;
        let path = "testfile.txt";

        // Write a file
        let write_fn: mlua::Function = fs_module.get("write").unwrap();
        write_fn
            .call::<()>((path.to_string(), "hello world".to_string()))
            .unwrap();

        // Read it back
        let read_fn: mlua::Function = fs_module.get("read").unwrap();
        let content: String = read_fn.call(path.to_string()).unwrap();
        assert_eq!(content, "hello world");

        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn test_fs_append() {
        let (_lua, (_, fs_module, _, _, _, _, _, _)) = setup_lua().await;
        let path = "appendfile.txt";

        // Write initial
        let write_fn: mlua::Function = fs_module.get("write").unwrap();
        write_fn
            .call::<()>((path.to_string(), "first".to_string()))
            .unwrap();

        // Append
        let append_fn: mlua::Function = fs_module.get("append").unwrap();
        append_fn
            .call::<()>((path.to_string(), " second".to_string()))
            .unwrap();

        // Read back
        let read_fn: mlua::Function = fs_module.get("read").unwrap();
        let content: String = read_fn.call(path.to_string()).unwrap();
        assert_eq!(content, "first second");

        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn test_env_get_set_remove() {
        let (_lua, (_, _, env_module,  _, _, _, _, _)) = setup_lua().await;

        let set_fn: mlua::Function = env_module.get("set").unwrap();
        set_fn
            .call::<()>(("TEST_ENV".to_string(), "value".to_string()))
            .unwrap();

        let get_fn: mlua::Function = env_module.get("get").unwrap();
        let value: Option<String> = get_fn.call("TEST_ENV".to_string()).unwrap();
        assert_eq!(value, Some("value".to_string()));

        let remove_fn: mlua::Function = env_module.get("remove").unwrap();
        remove_fn.call::<()>("TEST_ENV".to_string()).unwrap();

        let value: Option<String> = get_fn.call("TEST_ENV".to_string()).unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_shell_exec() {
        let (_lua, (_, _, _, shell_module, _, _, _, _)) = setup_lua().await;
        let exec_fn: mlua::Function = shell_module.get("exec").unwrap();

        #[cfg(unix)]
        let result: Table = exec_fn.call("echo hello").unwrap();
        #[cfg(windows)]
        let result: Table = exec_fn.call("echo hello").unwrap();

        let code: i64 = result.get("code").unwrap();
        let stdout: String = result.get("stdout").unwrap();

        assert_eq!(code, 0);
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_time_now_and_sleep() {
        let (_lua, (_, _, _, _, time_module, _, _, _)) = setup_lua().await;

        let now_fn: mlua::Function = time_module.get("now").unwrap();
        let formatted: String = now_fn.call("%Y-%m-%d".to_string()).unwrap();
        assert!(formatted.contains('-'));
    }

    #[tokio::test]
    async fn test_json_encode_decode() {
        let (lua, (_, _, _, _, _, json_module, _, _)) = setup_lua().await;

        let encode_fn: mlua::Function = json_module.get("encode").unwrap();
        let decode_fn: mlua::Function = json_module.get("decode").unwrap();

        let table = lua.create_table().unwrap();
        table.set("foo", "bar").unwrap();

        let json: String = encode_fn.call(table.clone()).unwrap();
        assert!(json.contains("foo"));

        let decoded: Table = decode_fn.call(json).unwrap();
        let foo: String = decoded.get("foo").unwrap();
        assert_eq!(foo, "bar");
    }
}