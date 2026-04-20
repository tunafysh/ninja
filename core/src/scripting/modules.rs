use chrono::prelude::*;
use log::{debug, error, info, warn};
use mlua::{ExternalError, Lua, LuaSerdeExt, Result, Table};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

mod env_module;
mod fs_module;
mod ninja_module;
mod proc_module;
mod shared;
mod shell_module;

pub(crate) use env_module::make_env_module;
pub(crate) use fs_module::make_fs_module;
pub(crate) use ninja_module::make_ninja_module;
pub(crate) use proc_module::make_proc_module;
use shared::{FetchArgs, http_download, http_request};
pub(crate) use shell_module::make_shell_module;

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
    let proc_module = make_proc_module(lua, cwd)?;

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

    http_module.set(
        "fetch",
        lua.create_async_function(|lua, (url, headers, method, body): FetchArgs| async move {
            debug!(
                "http.fetch: url='{}', headers={:?}, method={:?}, body={:?}",
                url, headers, method, body
            );
            let method = method.unwrap_or_else(|| "GET".to_string());
            let (status, response_body) = http_request(&method, &url, body, headers).await?;
            debug!(
                "http.fetch: url='{}' -> status={}, body_len={}",
                url,
                status,
                response_body.len()
            );
            let result_table = lua.create_table()?;
            result_table.set("status", status)?;
            result_table.set("body", response_body)?;
            Ok(result_table)
        })?,
    )?;

    http_module.set(
        "download",
        lua.create_async_function({
            let cwd_buf = shared::canonicalize_cwd(cwd);
            move |_, (url, dest): (String, PathBuf)| {
                let value = cwd_buf.clone();
                async move {
                    debug!("http.download: url='{}', dest='{}'", url, dest.display());
                    let bytes = http_download(&url).await?;
                    debug!("http.download: url='{}' -> {} bytes", url, bytes.len());

                    let dest = if let Some(cwd) = value.clone() {
                        cwd.join(dest)
                    } else {
                        dest
                    };

                    if let Some(parent) = dest.parent()
                        && let Err(e) = fs::create_dir_all(parent)
                    {
                        error!(
                            "http.download: failed to create parent directories for '{}': {}",
                            dest.display(),
                            e
                        );
                        return Err(mlua::Error::external(format!(
                            "Failed to create parent directories for '{}': {}",
                            dest.display(),
                            e
                        )));
                    }

                    match fs::write(&dest, &bytes) {
                        Ok(_) => {
                            debug!("http.download: successfully wrote to '{}'", dest.display());
                            Ok(())
                        }
                        Err(e) => {
                            error!(
                                "http.download: failed to write to '{}': {}",
                                dest.display(),
                                e
                            );
                            Err(mlua::Error::external(format!(
                                "Failed to write to '{}': {}",
                                dest.display(),
                                e
                            )))
                        }
                    }
                }
            }
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
