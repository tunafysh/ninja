use super::shared::canonicalize_cwd;
use log::debug;
use mlua::{Lua, Result, Table};
use std::{env, path::Path};

pub(crate) fn make_env_module(lua: &Lua, base_cwd: Option<&Path>) -> Result<Table> {
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
