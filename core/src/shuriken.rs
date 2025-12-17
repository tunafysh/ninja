use crate::util::kill_process_by_pid;
use crate::{
    scripting::NinjaEngine,
    templater::Templater,
    types::{FieldValue, PlatformPath},
};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{fs, process::Command};

pub fn make_admin_command(bin: &str, args: Option<&[String]>, cwd: Option<&Path>) -> Command {
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(bin);
        if let Some(a) = args {
            cmd.args(a);
        }
        if let Some(c) = cwd {
            cmd.current_dir(c);
        }
        cmd
    }

    #[cfg(target_os = "macos")]
    {
        use shell_escape;
        // osascript will trigger the GUI "enter password" dialog
        let mut cmd = Command::new("osascript");
        cmd.arg("-e").arg(format!(
            "do shell script \"{} {}\" with administrator privileges",
            shell_escape::escape(bin.into()),
            args.unwrap_or(&vec![]).join(" ")
        ));
        if let Some(c) = cwd {
            cmd.current_dir(c);
        }
        cmd
    }

    #[cfg(target_os = "windows")]
    {
        // runas triggers UAC, -PassThru returns the Process object, we extract .Id
        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile");
        cmd.arg("-NonInteractive");
        cmd.arg("-Command");

        // build PowerShell one-liner
        let mut script = format!("(Start-Process '{}' -Verb RunAs -PassThru", bin);

        if let Some(a) = args {
            // use array syntax for multiple args
            let joined = a.join("','");
            script.push_str(&format!(" -ArgumentList @('{}')", joined));
        }

        if let Some(c) = cwd
            && let Some(cwd_str) = c.to_str()
        {
            script.push_str(&format!(" -WorkingDirectory '{}'", cwd_str));
        }

        script.push_str(").Id"); // return PID

        cmd.arg(script);
        cmd
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tool {
    pub name: String,
    pub script: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenConfig {
    #[serde(rename = "config-path")]
    pub config_path: PathBuf,
    pub options: Option<HashMap<String, FieldValue>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShurikenMetadata {
    pub name: String,
    pub id: String,
    pub version: String,
    pub management: Option<ManagementType>,
    #[serde(rename = "type")]
    pub shuriken_type: String,
    #[serde(rename = "require-admin")]
    pub require_admin: bool,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(tag = "type")] // tag field determines the variant
pub enum ManagementType {
    #[serde(rename = "native")]
    Native {
        #[serde(rename = "bin-path")]
        bin_path: PlatformPath,
        args: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<PlatformPath>,
    },
    #[serde(rename = "script")]
    Script {
        #[serde(rename = "script-path")]
        script_path: PathBuf,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogsConfig {
    #[serde(rename = "log-path")]
    pub log_path: PathBuf,
}

fn kill_process_by_pid_and_start_time(pid: u32, expected_start_time: u64) -> Result<bool> {
    if let Some(actual_start_time) = get_process_start_time(pid)
        && actual_start_time == expected_start_time
    {
        #[cfg(not(windows))]
        return Ok(kill_process_by_pid(pid));
        #[cfg(windows)]
        return kill_process_by_pid(pid);
    }
    Ok(false)
}

pub fn get_process_start_time(pid: u32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        let stat_path = format!("/proc/{}/stat", pid);
        let contents = fs::read_to_string(stat_path).ok()?;
        let fields: Vec<&str> = contents.split_whitespace().collect();
        if fields.len() > 21 {
            let start_ticks: u64 = fields[21].parse().ok()?;
            let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
            Some(start_ticks / clock_ticks)
        } else {
            None
        }
    }

    #[cfg(target_os = "macos")]
    {
        use libc::{PROC_PIDTASKALLINFO, proc_pidinfo};
        use std::mem::MaybeUninit;

        let mut info = MaybeUninit::<libc::proc_taskallinfo>::uninit();
        let size = std::mem::size_of::<libc::proc_taskallinfo>() as i32;
        let ret = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDTASKALLINFO,
                0,
                info.as_mut_ptr() as *mut _,
                size,
            )
        };
        if ret == size {
            let info = unsafe { info.assume_init() };
            Some(info.pbsd.pbi_start_tvsec as u64)
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION,
        };

        unsafe {
            let handle = match OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
                Ok(h) => h,
                Err(_) => return None,
            };
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            if GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user).is_err() {
                return None;
            }

            let ticks = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
            let secs_since_1601 = ticks / 10_000_000;
            let unix_epoch_offset = 11_644_473_600u64;
            Some(secs_since_1601.saturating_sub(unix_epoch_offset))
        }
    }
}

async fn atomic_write_json(path: &Path, value: &JsonValue) -> Result<(), String> {
    use tokio::fs;

    let tmp_path = path.with_extension("tmp");

    let data = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    fs::write(&tmp_path, data)
        .await
        .map_err(|e| format!("Failed to write tmp lockfile: {e}"))?;

    // Atomic-ish replace on most platforms
    std::fs::rename(&tmp_path, path).map_err(|e| format!("Failed to replace lockfile: {e}"))?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shuriken {
    #[serde(rename = "shuriken")]
    pub metadata: ShurikenMetadata,
    pub config: Option<ShurikenConfig>,
    pub logs: Option<LogsConfig>,
    pub tools: Option<Vec<Tool>>,
}

impl Shuriken {
    pub async fn start(
        &self,
        engine: Option<&NinjaEngine>,
        shuriken_dir: &Path,
    ) -> Result<(), String> {
        info!("Starting shuriken {}", self.metadata.name);

        let lock_dir = shuriken_dir.join(".ninja");
        tokio::fs::create_dir_all(&lock_dir)
            .await
            .map_err(|e| format!("Failed to create .ninja directory: {}", e))?;
        let lock_path = lock_dir.join("shuriken.lck");

        match &self.metadata.management {
            Some(ManagementType::Native {
                bin_path,
                args,
                cwd,
            }) => {
                let bin_str = bin_path.get_path();

                // Resolve working directory
                let resolved_cwd = if let Some(custom_cwd) = cwd {
                    let p = custom_cwd.get_path();
                    let path = Path::new(p);
                    if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        shuriken_dir.join(path)
                    }
                } else {
                    shuriken_dir.parent().unwrap_or(shuriken_dir).to_path_buf()
                };

                let mut cmd = if self.metadata.require_admin {
                    make_admin_command(bin_str, args.as_deref(), Some(&resolved_cwd))
                } else {
                    let mut c = Command::new(bin_str);
                    if let Some(args) = args {
                        c.args(args);
                    }
                    c.current_dir(&resolved_cwd);
                    c
                };

                let mut process = cmd
                    .spawn()
                    .map_err(|e| format!("Failed to spawn process: {}", e))?;

                let pid = process
                    .id()
                    .ok_or_else(|| "Failed to get PID".to_string())?;
                let start_time = get_process_start_time(pid)
                    .ok_or_else(|| "Failed to get start time".to_string())?;

                let lockfile_data = json!({
                    "name": self.metadata.name,
                    "type": "Native",
                    "pid": pid,
                    "start_time": start_time
                });

                atomic_write_json(&lock_path, &lockfile_data).await?;

                tokio::spawn(async move {
                    let _ = process.wait().await;
                });

                Ok(())
            }

            Some(ManagementType::Script { script_path }) => {
                if let Some(engine) = engine {
                    // Resolve script path consistently relative to the shuriken root dir
                    let full_script_path =
                        self.resolve_script_path(script_path.as_path(), shuriken_dir);

                    let stem = full_script_path
                        .file_stem()
                        .ok_or_else(|| "Invalid script path".to_string())?
                        .to_string_lossy()
                        .to_string();
                    let compiled_path = lock_dir.join(format!("{stem}.ns"));

                    engine
                        .execute_function("start", &compiled_path, Some(shuriken_dir))
                        .map_err(|e| format!("Script start failed: {}", e))?;

                    let lockfile_data = json!({
                        "name": self.metadata.name,
                        "type": "Script",
                    });

                    atomic_write_json(&lock_path, &lockfile_data).await?;
                }
                Ok(())
            }

            None => Ok(()),
        }
    }

    pub async fn configure(&self, root_path: PathBuf) -> anyhow::Result<()> {
        if let Some(ctx) = &self.config {
            let shuriken_fields = ctx.options.clone();
            let mut fields = HashMap::new();
            if let Some(partial_fields) = shuriken_fields {
                for (name, value) in partial_fields {
                    fields.insert(name, value);
                }
            }
            
            

            // Construct full path to the shuriken folder
            let shuriken_path = root_path.join("shurikens").join(&self.metadata.name);

            // Ensure the directory exists
            fs::create_dir_all(&shuriken_path).await?;

            // Initialize Templater with the fields and shuriken path
            let templater = Templater::new(fields, shuriken_path.clone())?;

            // Full path to write the generated config
            let config_full_path = shuriken_path.join(&ctx.config_path);

            // Ensure the parent directory of the config file exists
            if let Some(parent) = config_full_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            templater
                .generate_config(config_full_path)
                .await
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;
        }

        Ok(())
    }

    fn resolve_script_path(&self, script_path: &Path, shuriken_dir: &Path) -> PathBuf {
        if script_path.is_absolute() {
            script_path.to_path_buf()
        } else {
            shuriken_dir.join(script_path)
        }
    }

    pub async fn stop(
        &self,
        engine: Option<&NinjaEngine>,
        shuriken_dir: &Path,
    ) -> Result<(), String> {
        info!("Stopping shuriken {}", self.metadata.name);
        let lock_path = shuriken_dir.join(".ninja").join("shuriken.lck");

        match &self.metadata.management {
            Some(ManagementType::Native { .. }) => {
                if !lock_path.exists() {
                    info!("Lock file not found, shuriken may not be running");
                    return Ok(());
                }

                let lock_contents = tokio::fs::read_to_string(&lock_path)
                    .await
                    .map_err(|e| format!("Failed to read lockfile: {}", e))?;

                let lockdata: JsonValue = serde_json::from_str(&lock_contents)
                    .map_err(|e| format!("Invalid lockfile: {}", e))?;

                let pid: u32 = serde_json::from_value(lockdata["pid"].clone())
                    .map_err(|e| format!("Invalid PID: {}", e))?;
                let start_time: u64 = serde_json::from_value(lockdata["start_time"].clone())
                    .map_err(|e| format!("Invalid start_time: {}", e))?;
                let name: String = serde_json::from_value(lockdata["name"].clone())
                    .map_err(|e| format!("Invalid name: {}", e))?;

                // Only use PID + start_time; don't kill by name to avoid hitting the wrong process.
                if !kill_process_by_pid_and_start_time(pid, start_time)
                    .map_err(|e| e.to_string())?
                {
                    return Err(format!(
                        "Failed to terminate shuriken {} (PID {}, start_time {})",
                        name, pid, start_time
                    ));
                }

                if lock_path.exists() {
                    tokio::fs::remove_file(&lock_path)
                        .await
                        .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
                }
                Ok(())
            }

            Some(ManagementType::Script { script_path }) => {
                if let Some(engine) = engine {
                    let full_script_path =
                        self.resolve_script_path(script_path.as_path(), shuriken_dir);

                    let stem = full_script_path
                        .file_stem()
                        .ok_or_else(|| "Invalid script".to_string())?
                        .to_string_lossy()
                        .to_string();
                    let lock_dir = shuriken_dir.join(".ninja");
                    let compiled_path = lock_dir.join(format!("{stem}.ns"));

                    engine
                        .execute_function("stop", &compiled_path, Some(shuriken_dir))
                        .map_err(|e| format!("Script stop failed: {}", e))?;

                    if lock_path.exists() {
                        tokio::fs::remove_file(&lock_path)
                            .await
                            .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
                    }
                }
                Ok(())
            }

            None => Ok(()),
        }
    }
}
