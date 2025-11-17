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
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tokio::{fs, process::Command};

fn make_admin_command(bin: &str, args: Option<&[String]>) -> Command {
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(bin);
        if let Some(a) = args {
            cmd.args(a);
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

        script.push_str(").Id"); // return PID

        cmd.arg(script);
        cmd
    }
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
    pub management: ManagementType,
    #[serde(rename = "type")]
    pub shuriken_type: String,
    #[serde(rename = "add-path")]
    pub add_path: Option<PathBuf>,
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

fn kill_process_by_name(name: &str) -> bool {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    for process in sys.processes().values() {
        if process.name() == name && process.kill_with(Signal::Kill).unwrap_or(false) {
            return true;
        }
    }
    false
}

fn kill_process_by_pid_and_start_time(pid: u32, expected_start_time: u64) -> bool {
    if let Some(actual_start_time) = get_process_start_time(pid)
        && actual_start_time == expected_start_time
    {
        return kill_process_by_pid(pid);
    }
    false
}

pub fn kill_process_by_pid(pid_num: u32) -> bool {
    let pid = Pid::from_u32(pid_num);
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    if let Some(process) = sys.process(pid) {
        process.kill_with(Signal::Kill).unwrap_or(false)
    } else {
        false
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shuriken {
    #[serde(rename = "shuriken")]
    pub metadata: ShurikenMetadata,
    pub config: Option<ShurikenConfig>,
    pub logs: Option<LogsConfig>,
}

impl Shuriken {
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting shuriken {}", self.metadata.name);

        let management = &self.metadata.management; // borrow, not clone

        match management {
            ManagementType::Native { bin_path, args, .. } => {
                let bin_str = bin_path.get_path();

                let mut cmd = if self.metadata.require_admin {
                    make_admin_command(bin_str, args.as_deref())
                } else {
                    let mut c = Command::new(bin_str);
                    if let Some(args) = args {
                        c.args(args);
                    }
                    c
                };

                let mut process = cmd
                    .spawn()
                    .map_err(|e| format!("Failed to spawn process: {}", e))?;

                let pid = process
                    .id()
                    .ok_or_else(|| "Failed to get PID of spawned process".to_string())?;

                let start_time = get_process_start_time(pid)
                    .ok_or_else(|| "Failed to get process start time".to_string())?;

                let lockfile_data = json!({
                    "name": self.metadata.name,
                    "type": "Native",
                    "pid": Pid::from(pid as usize).as_u32(),
                    "start_time": start_time
                });

                let lock_str = serde_json::to_string(&lockfile_data)
                    .map_err(|e| format!("Failed to serialize lockfile data: {}", e))?;

                fs::write("shuriken.lck", lock_str)
                    .await
                    .map_err(|e| format!("Failed to write lockfile: {}", e))?;

                tokio::spawn(async move {
                    let _ = process.wait().await;
                });

                Ok(())
            }
            ManagementType::Script { script_path } => {
                let engine = NinjaEngine::new()
                    .map_err(|e| format!("Failed to create NinjaEngine: {}", e))?;

                engine.execute_function("start", script_path).map_err(|e| {
                    format!(
                        "Failed to execute function 'start' in script '{}': {}",
                        script_path.display(),
                        e
                    )
                })?;

                let lockfile_data = json!({
                    "name": self.metadata.name,
                    "type": "Script",
                });

                let lock_str = serde_json::to_string(&lockfile_data)
                    .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;

                fs::write("shuriken.lck", lock_str)
                    .await
                    .map_err(|e| format!("Failed to write shuriken.lck: {}", e))?;

                Ok(())
            }
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

    pub async fn stop(&self) -> Result<(), String> {
        info!("Stopping shuriken {}", self.metadata.name);

        let management = &self.metadata.management;

        match management {
            ManagementType::Native { .. } => {
                let lock_contents = fs::read_to_string("shuriken.lck")
                    .await
                    .map_err(|e| format!("Failed to read lockfile: {}", e))?;

                let lockdata: JsonValue = serde_json::from_str(&lock_contents)
                    .map_err(|e| format!("Failed to parse lockfile JSON: {}", e))?;

                let pid: u32 = serde_json::from_value(lockdata["pid"].clone())
                    .map_err(|e| format!("Invalid PID in lockfile: {}", e))?;
                let start_time: u64 = serde_json::from_value(lockdata["start_time"].clone())
                    .map_err(|e| format!("Invalid start_time in lockfile: {}", e))?;
                let name: String = serde_json::from_value(lockdata["name"].clone())
                    .map_err(|e| format!("Invalid name in lockfile: {}", e))?;

                // Try by name first
                if !kill_process_by_name(&name) {
                    // Fallback to PID + start time
                    if !kill_process_by_pid_and_start_time(pid, start_time) {
                        return Err(format!(
                            "Failed to terminate shuriken {} (PID {}, start_time {})",
                            name, pid, start_time
                        ));
                    }
                }

                if Path::new("shuriken.lck").exists() {
                    fs::remove_file("shuriken.lck")
                        .await
                        .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
                }

                Ok(())
            }
            ManagementType::Script { script_path } => {
                let engine = NinjaEngine::new()
                    .map_err(|e| format!("Failed to create NinjaEngine: {}", e))?;

                engine.execute_function("stop", script_path).map_err(|e| {
                    format!(
                        "Failed to execute 'stop' in script '{}': {}",
                        script_path.display(),
                        e
                    )
                })?;

                fs::remove_file("shuriken.lck")
                    .await
                    .map_err(|e| format!("Failed to remove lockfile: {}", e))?;

                Ok(())
            }
        }
    }
}
