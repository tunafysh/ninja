#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod shell_api {
    use std::process::{Command, Stdio};

    #[rquickjs::function]
    pub fn exec(command: String) -> Result<String, rquickjs::Error> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &command])
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
        };

        match output {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    Err(rquickjs::Error::new_from_js_message("shell", "engine",
                        format!("Command failed: {}", String::from_utf8_lossy(&output.stderr))))
                }
            }
            Err(e) => Err(rquickjs::Error::new_from_js_message("shell", "engine", format!("Failed to execute: {}", e)))
        }
    }

    #[rquickjs::function]
    pub fn spawn(command: String, args: Vec<String>) -> Result<u32, rquickjs::Error> {
        let mut cmd = Command::new(&command);
        cmd.args(&args);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        match cmd.spawn() {
            Ok(child) => Ok(child.id()),
            Err(e) => Err(rquickjs::Error::new_from_js_message("shell", "engine", format!("Failed to spawn: {}", e)))
        }
    }

    #[rquickjs::function]
    pub fn kill(pid: u32) -> Result<(), rquickjs::Error> {
        let command = if cfg!(target_os = "windows") {
            format!("taskkill /PID {} /F", pid)
        } else {
            format!("kill -9 {}", pid)
        };

        exec(command)?;
        Ok(())
    }

    #[rquickjs::function]
    pub fn background(command: String, args: Vec<String>) -> Result<u32, rquickjs::Error> {
        let mut cmd = Command::new(&command);
        cmd.args(&args);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        cmd.stdin(Stdio::null());

        match cmd.spawn() {
            Ok(child) => Ok(child.id()),
            Err(e) => Err(rquickjs::Error::new_from_js_message("shell", "engine", format!("Failed to spawn background: {}", e)))
        }
    }
}