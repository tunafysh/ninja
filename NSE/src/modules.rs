use log::{debug, error, info, warn};
use mlua::{ExternalError, Lua, LuaSerdeExt, Result, Table};
use ::reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Method};
use serde_json::Value;
use std::{env, fs, io::Write, path::Path, process, str::FromStr, sync::Arc};
use chrono::prelude::*;
use reqwest::blocking as reqwest;

fn make_request(url: String, method: Option<String>, headers: Option<Table>) -> Result<Table> {
    let lua = Lua::new();
    let method = match method {
        Some(e) => Method::from_str(&e),
        None => Method::from_str("get")
    };
    
    let method = match method {
        Ok(e) => e,
        Err(_) => {
            eprintln!("Invalid method type!");
            Method::GET
        }   
    };
    
    let lua_headers = match headers {
        Some(e) => e,
        None => lua.create_table()?
    };
    
    let mut headers = HeaderMap::new();
    
    for pair in lua_headers.pairs::<String, String>() {
        let (key, value) = pair?;

        let name = HeaderName::try_from(&key)
            .map_err(|e| mlua::Error::external(format!("Invalid header name '{}': {}", key, e)))?;

        let value = HeaderValue::try_from(&value)
            .map_err(|e| mlua::Error::external(format!("Invalid header name '{}': {}", value, e)))?;

        headers.insert(name, value);
    }
    
    let request = reqwest::Client::new().request(method, url)
        .headers(headers)
        .send();
    
    let request = match request {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to send request: {}", e);
            let arc_error = Arc::new(e) as Arc<dyn std::error::Error + Send + Sync>;
            return Err(mlua::Error::ExternalError(arc_error))
        }
    };
    
    let response_headers = lua.create_table()?;

    let request_headers = request.headers();
    
    for pair in request_headers {
        let (name, value) = pair;

        let value = match value.to_str() {
            Ok(e) => e.to_string(),
            Err(e) => {
                eprintln!("Failed to convert header value to string. Reason:{}",e);
                "".to_string()
            }
        };
        response_headers.set(name.to_string(), value)?;
    }

    let response_status = request.status().as_u16();

    let response_text = match &request.text() {
        Ok(e) => e.to_string(),
        Err(_) => {
            eprintln!("Failed to get response body.");
            "".to_string()
        }
    };
        
    let response = lua.create_table()?;
    
    response.set("status", response_status)?;
    response.set("body", response_text)?;
    response.set("status", response_headers)?;
    
    Ok(response)
}


pub fn make_modules(lua: &Lua) -> Result<(Table, Table, Table, Table, Table, Table, Table)>  {
    let fs_module = lua.create_table()?;
    let env_module = lua.create_table()?;
    let shell_module = lua.create_table()?;
    let time_module = lua.create_table()?;
    let json_module = lua.create_table()?;
    let http_module = lua.create_table()?;
    let log_module = lua.create_table()?;

    // ================= fs module =================

    fs_module.set("read_file",lua.create_function( |_, path: String| {
        Ok(fs::read_to_string(path)?)
    })?)?;

    fs_module.set("write_file",lua.create_function( |_, (path, content): (String, String)| {
        Ok(fs::write(path, content)?)
    })?)?;

    fs_module.set("append_file",lua.create_function( |_, (path, content): (String, String)| {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(path)?;

        Ok(file.write_all(content.as_bytes())?)
    })?)?;

    fs_module.set("remove_file",lua.create_function( |_, path: String| {
        Ok(fs::remove_file(path)?)
    })?)?;

    fs_module.set("create_dir",lua.create_function( |_, path: String| {
        Ok(fs::create_dir(path)?)
    })?)?;

    fs_module.set("read_dir",lua.create_function( |_, path: String| {
        let entries: std::io::Result<Vec<String>> = std::fs::read_dir(path)?
        .map(|entry| {
            entry
                .map(|e| e.file_name().into_string().unwrap_or_else(|os_str| {
                    format!("<invalid UTF-8: {:?}>", os_str)
                }))
        })
        .collect();

    match entries {
        Ok(files) => Ok(files),
        Err(e) => Err(mlua::Error::external(e)),
    }
    })?)?;

    fs_module.set("exists",lua.create_function( |_, path: String| {
        Ok(fs::exists(path)?)
    })?)?;

    fs_module.set("is_dir",lua.create_function( |_, path: String| {
        Ok(Path::new(&path).is_dir())
    })?)?;

    fs_module.set("exists",lua.create_function( |_, path: String| {
        Ok(Path::new(&path).is_file())
    })?)?;

    // ================= env module =================

    env_module.set("os", env::consts::OS)?;
    env_module.set("arch", env::consts::ARCH)?;

    env_module.set("get",lua.create_function( |_, key: String| {
        match env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(e) => {
                eprintln!("Failed to get environment variable, reason: {}",e);
                Ok(None)
            }
        }
    })?)?;

    env_module.set("set",lua.create_function( |_, (key, value): (String, String)| {
        unsafe {
            Ok(env::set_var(key, value))
        }
    })?)?;

    env_module.set("remove",lua.create_function( |_, key: String| {
        unsafe {
            Ok(env::remove_var(key))
        }
    })?)?;

    env_module.set("vars",lua.create_function( |lua, _: ()| {
        let table = lua.create_table()?;

        for (k, v) in env::vars() {
            table.set(k, v)?
        }
        Ok(table)
    })?)?;

    // ================= shell module =================

    shell_module.set("exec", lua.create_function(|lua, command: String| {
        let result_table = lua.create_table()?;
        let cmd = process::Command::new(command)
        .output()?;
    
        let exit_code = cmd.status.code();
        if cmd.status.success() {
            result_table.set("exit_code", 0)?;
        }
        else {
            result_table.set("exit_code", exit_code.unwrap_or(-1))?;
        }

        let (stdout, stderr) = (String::from_utf8_lossy(&cmd.stdout).to_string(), String::from_utf8_lossy(&cmd.stderr).to_string());
        result_table.set("stdout", stdout)?;
        result_table.set("stderr", stderr)?;

        Ok(result_table)
    })?)?;

    // ================= time module =================

    time_module.set("year", lua.create_function(|_, _:()| {
        Ok(Utc::now().year())
    })?)?;

    time_module.set("month", lua.create_function(|_, _:()| {
        Ok(Utc::now().month())
    })?)?;

    time_module.set("day", lua.create_function(|_, _:()| {
        Ok(Utc::now().day())
    })?)?;

    time_module.set("hour", lua.create_function(|_, format: bool| {
        let now = Utc::now();
        if format {
            // Return 12-hour clock hour (1-12)
            let (is_pm, hour) = now.hour12();
            Ok((hour, if is_pm { "PM" } else { "AM" }))
        } else {
            // Return 24-hour clock hour (0-23)
            Ok((now.hour(), ""))
        }
    })?)?;

    time_module.set("minute", lua.create_function(|_, _:()| {
        Ok(Utc::now().minute())
    })?)?;

    time_module.set("second", lua.create_function(|_, _:()| {
        Ok(Utc::now().second())
    })?)?;

    time_module.set("now", lua.create_function(|_, format: String| {
        Ok(Utc::now().format(format.as_str()).to_string())
    })?)?;

    // ================= json module =================

    json_module.set("encode", lua.create_function(|_, data: Table| {
        match serde_json::to_string(&data) {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_lua_err())
        }
    })?)?;

    json_module.set("decode", lua.create_function(|lua, json_string: String| {
        let json_value: Value = match serde_json::from_str(&json_string) {
            Ok(val) => val,
            Err(e) => format!("Invalid JSON: {}", e).into()
        };

        let lua_value = lua.to_value(&json_value)?;

        if let mlua::Value::Table(table) = lua_value {
            Ok(table)
        } else {
            Err(mlua::Error::DeserializeError("JSON did not represent an object or array — expected a table".to_string()))
        }

        
        
    })?)?;

    // ================= http module =================

    http_module.set("fetch", lua.create_function(|_, (url, method, headers): (String, Option<String>, Option<Table>)| {
        make_request(url, method, headers)
    })?)?;

    // ================= log module =================

    log_module.set("info", lua.create_function(|_, content: String|{
        Ok(info!("{}",content))
    })?)?;

    log_module.set("warn", lua.create_function(|_, content: String|{
        Ok(warn!("{}",content))
    })?)?;

    log_module.set("error", lua.create_function(|_, content: String|{
        Ok(error!("{}",content))
    })?)?;

    log_module.set("debug", lua.create_function(|_, content: String|{
        Ok(debug!("{}",content))
    })?)?;

    Ok((fs_module, env_module, shell_module, time_module, json_module, http_module, log_module))
}