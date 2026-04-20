use crate::utils::resolve_path;
use log::{debug, error, warn};
use mlua::{Lua, Result, Table};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) fn make_fs_module(lua: &Lua, cwd: Option<&Path>) -> Result<Table> {
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

    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "write",
            lua.create_function(move |_, (path, content): (PathBuf, String)| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.write: '{}' (len={})", resolved.display(), content.len());
                fs::write(&resolved, content).map_err(|e| {
                    error!("fs.write: failed for '{}': {}", resolved.display(), e);
                    mlua::Error::external(e)
                })
            })?,
        )?;
    }

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
                        error!("fs.append: failed to open '{}': {}", resolved.display(), e);
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

    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "remove",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.remove: '{}'", resolved.display());
                fs::remove_file(&resolved).map_err(|e| {
                    error!("fs.remove: failed for '{}': {}", resolved.display(), e);
                    mlua::Error::external(e)
                })?;
                Ok(())
            })?,
        )?;
    }

    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "create_dir",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.create_dir: '{}'", resolved.display());
                fs::create_dir_all(&resolved).map_err(|e| {
                    error!("fs.create_dir: failed for '{}': {}", resolved.display(), e);
                    mlua::Error::external(e)
                })?;
                Ok(())
            })?,
        )?;
    }

    {
        let fs_cwd = base_cwd.clone();
        fs_module.set(
            "read_dir",
            lua.create_function(move |_, path: PathBuf| {
                let resolved = resolve_with_cwd(&fs_cwd, &path);
                debug!("fs.read_dir: '{}'", resolved.display());
                let entries = fs::read_dir(&resolved).map_err(|e| {
                    error!("fs.read_dir: failed for '{}': {}", resolved.display(), e);
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
