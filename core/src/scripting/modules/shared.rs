use log::{debug, error, warn};
use mlua::{Error as LuaError, Result};
use regex;
use relative_path::RelativePath;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

pub(crate) type FetchArgs = (
    String,
    Option<HashMap<String, String>>,
    Option<String>,
    Option<String>,
);

#[cfg(windows)]
pub(crate) fn strip_windows_prefix(p: &Path) -> PathBuf {
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
pub(crate) fn strip_windows_prefix(p: &Path) -> PathBuf {
    p.to_path_buf()
}

pub(crate) async fn http_request(
    method: &str,
    url: &str,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
) -> Result<(u16, String)> {
    debug!(
        "http_request: method='{}', url='{}', body_len={}",
        method,
        url,
        body.as_ref().map(|b| b.len()).unwrap_or(0)
    );

    let client = reqwest::Client::new();
    if let Some(headers) = &headers {
        for (k, v) in headers.iter() {
            debug!("http_request: header '{}: {}'", k, v);
        }
    }
    let request_builder = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => {
            error!(
                "http_request: unsupported method '{}'. Defaulting to GET.",
                method
            );
            client.get(url)
        }
    };

    let request_builder = if let Some(body) = body {
        request_builder.body(body)
    } else {
        request_builder
    };

    let response = request_builder.send().await.map_err(|e| {
        error!(
            "http_request: request failed for '{} {}': {}",
            method, url, e
        );
        LuaError::external(e)
    })?;

    let status = response.status().as_u16();
    let text = response.text().await.map_err(|e| {
        error!(
            "http_request: failed to read response text for '{} {}': {}",
            method, url, e
        );
        LuaError::external(e)
    })?;

    debug!(
        "http_request: completed '{} {}' with status {}, response_len={}",
        method,
        url,
        status,
        text.len()
    );

    Ok((status, text))
}

pub(crate) async fn http_download(url: &str) -> Result<Vec<u8>> {
    debug!("http_download: url='{}'", url);
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            error!("http_download: request failed for '{}': {}", url, e);
            LuaError::external(e)
        })?
        .bytes()
        .await
        .map_err(|e| {
            error!("http_download: failed to read bytes for '{}': {}", url, e);
            LuaError::external(e)
        })?
        .to_vec();
    debug!(
        "http_download: completed '{}', bytes_len={}",
        url,
        response.len()
    );
    Ok(response)
}

pub(crate) fn canonicalize_cwd(base: Option<&Path>) -> Option<PathBuf> {
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
        debug!("canonicalize_cwd: final result = '{}'", stripped.display());
        stripped
    });

    debug!(
        "canonicalize_cwd: returning {:?}",
        result.as_ref().map(|p| p.display().to_string())
    );
    result
}

pub(crate) fn resolve_spawn_command(
    command: &str,
    cwd: Option<&Path>,
    use_backslash: Option<bool>,
) -> Result<String> {
    let use_backslash = use_backslash.unwrap_or(false);

    if command.is_empty() {
        return Ok(command.to_string());
    }

    let Some(cwd) = cwd else {
        return Ok(command.to_string());
    };

    let re = regex::Regex::new(r#"(\.\.?[/\\][^\s"'`]+)"#).map_err(LuaError::external)?;

    let result = re.replace_all(command, |caps: &regex::Captures| {
        let rel_path = &caps[0];

        debug!("resolve_spawn_command: found relative path '{}'", rel_path);

        let normalized = rel_path.replace('\\', "/");
        let abs = RelativePath::new(&normalized).to_logical_path(cwd);

        let mut path_str = abs.to_string_lossy().to_string();

        if use_backslash {
            path_str = path_str.replace('/', "\\");
        }

        path_str
    });

    Ok(result.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_windows_prefix_with_prefix() {
        #[cfg(windows)]
        {
            let path = PathBuf::from(r"\\?\C:\test\path");
            let result = strip_windows_prefix(&path);
            assert_eq!(result, PathBuf::from(r"C:\test\path"));
        }
    }

    #[test]
    fn test_strip_windows_prefix_without_prefix() {
        let path = PathBuf::from("/test/path");
        let result = strip_windows_prefix(&path);
        assert_eq!(result, path);
    }

    #[test]
    fn test_canonicalize_cwd_none() {
        let result = canonicalize_cwd(None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_spawn_command_empty() {
        let result = resolve_spawn_command("", None, None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_spawn_command_simple() {
        let use_backslash = cfg!(windows);
        let result = resolve_spawn_command("ls -la", None, Some(use_backslash)).unwrap();
        assert!(result.contains("ls"));
    }

    #[test]
    fn test_resolve_spawn_command_with_path() {
        let use_backslash = cfg!(windows);
        let result = resolve_spawn_command("./myapp arg1 arg2", None, Some(use_backslash)).unwrap();
        assert!(result.contains("./myapp") || result.contains("myapp"));
    }

    #[test]
    fn test_resolve_spawn_command_multiple_relative_paths() {
        let temp_dir = env::temp_dir();
        let use_backslash = cfg!(windows);
        let result = resolve_spawn_command(
            "./myapp ./input.txt ./output.txt",
            Some(&temp_dir),
            Some(use_backslash),
        )
        .unwrap();

        assert!(
            !result.contains("./"),
            "Result should not contain './' after resolution: {}",
            result
        );

        assert!(result.contains("myapp"));
        assert!(result.contains("input.txt"));
        assert!(result.contains("output.txt"));
    }

    #[test]
    fn test_resolve_spawn_command_backslash_paths() {
        let temp_dir = env::temp_dir();
        let input = r".\myapp.exe .\file.txt";
        let result = resolve_spawn_command(input, Some(&temp_dir), Some(true)).unwrap();

        assert!(
            !result.contains(r".\"),
            "Result should not contain '.\\' after resolution: {}",
            result
        );

        assert!(result.contains("myapp"));
        assert!(result.contains("file.txt"));
    }
}