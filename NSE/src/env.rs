#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod env_api {
    use rquickjs::{Ctx, Result, Array, Object};
    use std::env;

    /// Get environment variable value
    #[rquickjs::function]
    pub fn get(name: String) -> Option<String> {
        env::var(name).ok()
    }

    /// Set environment variable
    #[rquickjs::function]
    pub fn set(name: String, value: String) {
        unsafe {
            env::set_var(name, value);
        }
    }

    /// Remove environment variable
    #[rquickjs::function]
    pub fn remove(name: String) {
        unsafe {
            env::remove_var(name);
        }
    }

    /// Get all environment variables as an object
    #[rquickjs::function]
    pub fn all(ctx: Ctx<'_>) -> Result<Object<'_>> {
        let obj = Object::new(ctx)?;
        
        for (key, value) in env::vars() {
            obj.set(key, value)?;
        }
        
        Ok(obj)
    }

    /// Get command line arguments
    #[rquickjs::function]
    pub fn args(ctx: Ctx<'_>) -> Result<Array<'_>> {
        let args = Array::new(ctx)?;
        
        for (i, arg) in env::args().enumerate() {
            args.set(i, arg)?;
        }
        
        Ok(args)
    }

    /// Get current working directory
    #[rquickjs::function]
    pub fn cwd() -> Result<String> {
        env::current_dir()
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|e| rquickjs::Error::new_from_js_message("env", "engine", format!("Failed to get current directory: {}", e)))
    }

    /// Change current working directory
    #[rquickjs::function]
    pub fn chdir(path: String) -> Result<()> {
        env::set_current_dir(path)
            .map_err(|e| rquickjs::Error::new_from_js_message("env", "engine", format!("Failed to change directory: {}", e)))
    }

    /// Get home directory
    #[rquickjs::function]
    pub fn home() -> Option<String> {
        env::var("HOME").ok()
            .or_else(|| env::var("USERPROFILE").ok())
    }

    /// Get temporary directory
    #[rquickjs::function]
    pub fn temp() -> String {
        env::temp_dir().to_string_lossy().to_string()
    }

    /// Get user name
    #[rquickjs::function]
    pub fn user() -> Option<String> {
        env::var("USER").ok()
            .or_else(|| env::var("USERNAME").ok())
    }

    /// Get hostname
    #[rquickjs::function]
    pub fn hostname() -> Option<String> {
        env::var("HOSTNAME").ok()
            .or_else(|| env::var("COMPUTERNAME").ok())
    }

    /// Get PATH environment variable as array
    #[rquickjs::function]
    pub fn path(ctx: Ctx<'_>) -> Result<Array<'_>> {
        let path_var = env::var("PATH").unwrap_or_default();
        let paths = Array::new(ctx)?;
        
        let separator = if cfg!(windows) { ";" } else { ":" };
        
        for (i, path) in path_var.split(separator).enumerate() {
            paths.set(i, path)?;
        }
        
        Ok(paths)
    }

    /// Check if environment variable exists
    #[rquickjs::function]
    pub fn has(name: String) -> bool {
        env::var(name).is_ok()
    }

    /// Get environment variable with default value
    #[rquickjs::function]
    pub fn get_or(name: String, default: String) -> String {
        env::var(name).unwrap_or(default)
    }

    /// Get environment variables matching a pattern
    #[rquickjs::function]
    pub fn filter(ctx: Ctx<'_>, pattern: String) -> Result<Object<'_>> {
        let obj = Object::new(ctx)?;
        
        for (key, value) in env::vars() {
            if key.contains(&pattern) {
                obj.set(&key, value)?;
            }
        }
        
        Ok(obj)
    }
}