use log::info;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tokio::{fs, process::Command};
use std::{collections::HashMap, path::Path};
use crate::config::{ConfigParam, LogsConfig, MaintenanceType, ShurikenConfig};
use ninja_engine::NinjaEngine;
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

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
        use libc::{proc_pidinfo, PROC_PIDTASKALLINFO};
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
        use windows::Win32::System::Threading::{OpenProcess, GetProcessTimes, PROCESS_QUERY_INFORMATION};
        use windows::Win32::Foundation::FILETIME;

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
    pub shuriken: ShurikenConfig,
    pub config: Option<HashMap<String, ConfigParam>>,
    pub logs: Option<LogsConfig>,
}

impl Shuriken {
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting shuriken {}", self.shuriken.name);

        let maintenance = &self.shuriken.maintenance; // borrow, not clone

        match maintenance {
            MaintenanceType::Native { bin_path, args, .. } => {
                let bin_str = bin_path.get_path();

                let mut cmd = Command::new(bin_str);

                if let Some(args) = args {
                    cmd.args(args);
                }

                let mut process = cmd.spawn()
                    .map_err(|e| format!("Failed to spawn process: {}", e))?;

                let pid = process.id()
                    .ok_or_else(|| "Failed to get PID of spawned process".to_string())?;

                let start_time = get_process_start_time(pid)
                    .ok_or_else(|| "Failed to get process start time".to_string())?;

                let lockfile_data = json!({
                    "name": self.shuriken.name,
                    "type": "Native",
                    "pid": Pid::from(pid as usize).as_u32(),
                    "start_time": start_time
                });

                let lock_str = serde_json::to_string(&lockfile_data)
                    .map_err(|e| format!("Failed to serialize lockfile data: {}", e))?;

                fs::write("shuriken.lck", lock_str)
                    .await
                    .map_err(|e| format!("Failed to write lockfile: {}", e))?;

                tokio::spawn(async move { let _ = process.wait().await; });

                Ok(())
            }
            MaintenanceType::Script { script_path } => {
                let engine = NinjaEngine::new()
                    .map_err(|e| format!("Failed to create NinjaEngine: {}", e))?;

                engine.execute_function("start", script_path, None)
                    .map_err(|e| format!("Failed to execute 'start' in script '{}': {}", script_path.display(), e))?;

                let lockfile_data = json!({
                    "name": self.shuriken.name,
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

    pub async fn stop(&self) -> Result<(), String> {
        info!("Stopping shuriken {}", self.shuriken.name);

        let maintenance = &self.shuriken.maintenance;

        match maintenance {
            MaintenanceType::Native { .. } => {
                let lock_contents = fs::read_to_string("shuriken.lck")
                    .await
                    .map_err(|e| format!("Failed to read lockfile: {}", e))?;

                let lockdata: Value = serde_json::from_str(&lock_contents)
                    .map_err(|e| format!("Failed to parse lockfile JSON: {}", e))?;

                let pid: u32 = serde_json::from_value(lockdata["pid"].clone())
                    .map_err(|e| format!("Invalid PID in lockfile: {}", e))?;

                if !kill_process_by_pid(pid) {
                    return Err(format!("Failed to terminate process with PID {}", pid));
                }
                
                if Path::new("shuriken.lck").exists() {
                    fs::remove_file("shuriken.lck")
                    .await
                    .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
                };

                Ok(())
            }
            MaintenanceType::Script { script_path } => {
                let engine = NinjaEngine::new()
                    .map_err(|e| format!("Failed to create NinjaEngine: {}", e))?;

                engine.execute_function("stop", script_path, None)
                    .map_err(|e| format!("Failed to execute 'stop' in script '{}': {}", script_path.display(), e))?;

                fs::remove_file("shuriken.lck")
                    .await
                    .map_err(|e| format!("Failed to remove lockfile: {}", e))?;

                Ok(())
            }
        }
    }
}