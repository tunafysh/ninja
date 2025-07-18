#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod sys_api {
    use rquickjs::{Ctx, Result, Object};

    /// Get the current platform/OS
    #[rquickjs::function]
    pub fn platform() -> String {
        if cfg!(target_os = "windows") {
            "windows".to_string()
        } else if cfg!(target_os = "macos") {
            "macos".to_string()
        } else if cfg!(target_os = "linux") {
            "linux".to_string()
        } else if cfg!(target_os = "freebsd") {
            "freebsd".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Get list of running processes (process names and PIDs)
    #[rquickjs::function]
    pub fn processes() -> String {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        struct Process {
            pid: u32,
            name: String,
        }

        let mut processes = Vec::<Process>::new();

        #[cfg(target_os = "linux")]
        {
            use std::fs;

            if let Ok(entries) = fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.chars().all(|c| c.is_ascii_digit()) {
                            if let Ok(pid) = file_name.parse::<u32>() {
                                let comm_path = format!("/proc/{}/comm", pid);
                                if let Ok(comm) = fs::read_to_string(&comm_path) {
                                    processes.push(Process {
                                        pid,
                                        name: comm.trim().to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("tasklist")
                .args(["/fo", "csv", "/nh"])
                .output()
                {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        for line in stdout.lines() {
                            let fields: Vec<&str> = line.split(',').collect();
                            if fields.len() >= 2 {
                                let name = fields[0].trim_matches('"');
                                if let Ok(pid) = fields[1].trim_matches('"').parse::<u32>() {
                                    processes.push(Process {
                                        pid,
                                        name: name.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("ps")
                .args(["-eo", "pid,comm"])
                .output()
                {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        for line in stdout.lines().skip(1) { // Skip header line
                            let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                if let Ok(pid) = parts[0].parse::<u32>() {
                                    let name = parts[1].trim();
                                    // Extract just the command name without path
                                    let command_name = name.split('/').last().unwrap_or(name);
                                    processes.push(Process {
                                        pid,
                                        name: command_name.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
        }

        serde_json::to_string(&processes).unwrap_or_else(|_| "[]".to_string())
    }


    /// Get memory information
    #[rquickjs::function]
    pub fn memory(ctx: Ctx<'_>) -> Result<Object<'_>> {
        let obj = Object::new(ctx)?;
        
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                let mut total = 0u64;
                let mut available = 0u64;
                let mut free = 0u64;
                
                for line in meminfo.lines() {
                    if let Some(value) = line.strip_prefix("MemTotal:") {
                        total = parse_mem_value(value);
                    } else if let Some(value) = line.strip_prefix("MemAvailable:") {
                        available = parse_mem_value(value);
                    } else if let Some(value) = line.strip_prefix("MemFree:") {
                        free = parse_mem_value(value);
                    }
                }
                
                obj.set("total", total)?;
                obj.set("available", available)?;
                obj.set("free", free)?;
                obj.set("used", total.saturating_sub(available))?;
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Default values for non-Linux systems
            obj.set("total", 0u64)?;
            obj.set("available", 0u64)?;
            obj.set("free", 0u64)?;
            obj.set("used", 0u64)?;
        }
        
        Ok(obj)
    }

    /// Get system uptime in seconds
    #[rquickjs::function]
    pub fn uptime() -> Result<u64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
                if let Some(uptime_part) = uptime_str.split_whitespace().next() {
                    if let Ok(uptime_f64) = uptime_part.parse::<f64>() {
                        return Ok(uptime_f64 as u64);
                    }
                }
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Fallback: calculate from system boot time if available
            // For now, return 0 for non-Linux systems
            return Ok(0);
        }
        
    }

    #[cfg(target_os = "linux")]
    fn parse_mem_value(value: &str) -> u64 {
        value.trim()
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024 // Convert KB to bytes
    }
}
