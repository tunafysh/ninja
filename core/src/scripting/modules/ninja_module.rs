use crate::{common::types::ShurikenState, manager::ShurikenManager};
use mlua::{Either, Error as LuaError, Lua, Result, Table};
use std::{path::PathBuf, sync::Arc};

pub(crate) fn make_ninja_module(lua: &Lua, manager: ShurikenManager) -> Result<Table> {
    let ninja_module = lua.create_table()?;
    let mgr = Arc::new(manager.clone());

    ninja_module.set(
        "start",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, name: String| {
                let mgr = mgr.clone();

                async move {
                    mgr.start(&name).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "stop",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, name: String| {
                let mgr = mgr.clone();

                async move {
                    mgr.stop(&name).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "refresh",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, _: ()| {
                let mgr = mgr.clone();

                async move {
                    mgr.refresh().await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "list",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |lua, with_state: bool| {
                let mgr = mgr.clone();

                async move {
                    let result = mgr.list(with_state).await.map_err(LuaError::external)?;
                    let table = lua.create_table()?;

                    match result {
                        Either::Left(vec) => {
                            let vec: Vec<(String, ShurikenState)> = vec;
                            for (i, (name, state)) in vec.into_iter().enumerate() {
                                let row = lua.create_table()?;
                                row.set("name", name)?;
                                row.set("state", format!("{:?}", state))?;
                                table.set(i + 1, row)?;
                            }
                        }
                        Either::Right(vec) => {
                            let vec: Vec<String> = vec;
                            for (i, name) in vec.into_iter().enumerate() {
                                table.set(i + 1, name)?;
                            }
                        }
                    }

                    Ok(table)
                }
            }
        })?,
    )?;

    ninja_module.set(
        "configure",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, name: String| {
                let mgr = mgr.clone();

                async move {
                    mgr.configure(&name).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "lockpick",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, name: String| {
                let mgr = mgr.clone();

                async move {
                    mgr.lockpick(&name).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "install",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, path: String| {
                let mgr = mgr.clone();

                async move {
                    let path = PathBuf::from(path);
                    mgr.install(path.as_path()).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    ninja_module.set(
        "get_projects",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, _: ()| {
                let mgr = mgr.clone();

                async move {
                    let projects = mgr.get_projects().await?;
                    Ok(projects)
                }
            }
        })?,
    )?;

    ninja_module.set(
        "remove",
        lua.create_async_function({
            let mgr = mgr.clone();

            move |_, name: String| {
                let mgr = mgr.clone();

                async move {
                    mgr.remove(&name).await?;
                    Ok(())
                }
            }
        })?,
    )?;

    Ok(ninja_module)
}
