pub mod download;

use anyhow::{Result, anyhow};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn get_http_port() -> Result<u16> {
    let apache_conf = "shurikens/Apache/conf/httpd.conf";
    let nginx_conf = "shurikens/Nginx/nginx.conf";

    if Path::new(apache_conf).exists() {
        return parse_apache_port(apache_conf);
    }

    if Path::new(nginx_conf).exists() {
        return parse_nginx_port(nginx_conf);
    }

    Err(anyhow!("No Apache or Nginx shuriken found in ./shurikens"))
}

fn parse_apache_port(path: &str) -> Result<u16> {
    let file = fs::read_to_string(path)?;
    let re = Regex::new(r#"(?mi)^\s*Listen\s+([0-9]+)"#)?;

    if let Some(cap) = re.captures(&file) {
        return Ok(cap[1].parse()?);
    }

    Err(anyhow!(
        "Apache config exists but contains no Listen directive"
    ))
}

fn parse_nginx_port(path: &str) -> Result<u16> {
    let file = fs::read_to_string(path)?;
    let re = Regex::new(r#"(?mi)^\s*listen\s+([0-9]+)"#)?;

    if let Some(cap) = re.captures(&file) {
        return Ok(cap[1].parse()?);
    }

    Err(anyhow!(
        "Nginx config exists but contains no listen directive"
    ))
}

pub fn resolve_path(virtual_cwd: &Path, path: &PathBuf) -> PathBuf {
    let p = Path::new(path);

    if p.is_absolute() {
        p.to_path_buf()
    } else {
        virtual_cwd.join(p)
    }
}

#[cfg(windows)]
pub fn kill_process_by_pid(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
    };

    unsafe {
        // First check if the process exists by trying to query it
        let query_handle = match OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => {
                // Process doesn't exist or we don't have access - either way, we can't kill it
                return false;
            }
        };

        if query_handle.is_invalid() {
            return false;
        }

        // Close the query handle before trying to terminate
        let _ = CloseHandle(query_handle);

        // Now try to open with terminate rights
        let terminate_handle = match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(h) => h,
            Err(_) => {
                // Process exists but we can't terminate it (permissions issue)
                return false;
            }
        };

        if terminate_handle.is_invalid() {
            return false;
        }

        // Attempt termination with exit code 1
        let terminate_result = TerminateProcess(terminate_handle, 1);
        let _ = CloseHandle(terminate_handle);

        // Return true if termination succeeded, false otherwise
        terminate_result.is_ok()
    }
}

#[cfg(windows)]
pub fn kill_process_by_name(name: &str) -> bool {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    let target = name.to_ascii_lowercase();

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if snapshot.is_invalid() {
            return false;
        }

        let mut entry = PROCESSENTRY32W::default();

        let mut any_killed = false;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                // exe file name is in szExeFile (null-terminated UTF-16)
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());

                let exe_os: OsString = OsString::from_wide(&entry.szExeFile[..len]);
                let exe = exe_os.to_string_lossy().to_string();

                if exe.to_ascii_lowercase() == target {
                    let pid = entry.th32ProcessID;
                    if kill_process_by_pid(pid) {
                        any_killed = true;
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(HANDLE(snapshot.0));
        any_killed
    }
}

#[cfg(unix)]
pub fn kill_process_by_pid(pid: u32) -> bool {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let pid = Pid::from_raw(pid as i32);

    // First check if the process exists using signal 0 (no-op signal)
    match kill(pid, None) {
        Ok(_) => {
            // Process exists, try SIGTERM first for graceful shutdown
            match kill(pid, Signal::SIGTERM) {
                Ok(_) => true,
                Err(Errno::EPERM) => {
                    // Permission denied - process exists but we can't kill it
                    false
                }
                Err(_) => {
                    // Other error (like ESRCH - process disappeared), try SIGKILL as fallback
                    kill(pid, Signal::SIGKILL).is_ok()
                }
            }
        }
        Err(Errno::ESRCH) => {
            // Process doesn't exist
            false
        }
        Err(Errno::EPERM) => {
            // Process exists but we don't have permission even to check it
            // Try SIGKILL anyway in case we have partial permissions
            kill(pid, Signal::SIGKILL).is_ok()
        }
        Err(_) => {
            // Other errors - process likely doesn't exist or is inaccessible
            false
        }
    }
}

#[cfg(unix)]
pub fn kill_process_by_name(name: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::io::Read;
        use std::path::Path;

        let target = name.to_string();
        let mut any_killed = false;

        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name_str = match file_name.to_string_lossy().parse::<u32>() {
                    Ok(_) => file_name.to_string_lossy().to_string(),
                    Err(_) => continue, // skip non-numeric (not a PID dir)
                };

                let pid: u32 = match file_name_str.parse() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                let comm_path = entry.path().join("comm"); // /proc/<pid>/comm

                if !Path::new(&comm_path).exists() {
                    continue;
                }

                let mut comm_file = match fs::File::open(&comm_path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                let mut comm = String::new();
                if comm_file.read_to_string(&mut comm).is_err() {
                    continue;
                }

                let comm = comm.trim(); // usually just "bash" or "node", etc.

                if comm == target && kill_process_by_pid(pid) {
                    any_killed = true;
                }
            }
        }

        any_killed
    }

    // Fallback for macOS / other Unix: use `ps` to find matches
    #[cfg(not(target_os = "linux"))]
    {
        use std::process::Command;

        let target = name.to_string();
        let output = match Command::new("ps")
            .args(["-axo", "pid,comm"]) // pid + command name
            .output()
        {
            Ok(o) => o,
            Err(_) => return false,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut any_killed = false;

        for line in stdout.lines().skip(1) {
            // format: "  1234 /usr/bin/myprog"
            let mut parts = line.split_whitespace();
            let pid_str = match parts.next() {
                Some(p) => p,
                None => continue,
            };
            let comm = parts.next().unwrap_or("");

            let pid: u32 = match pid_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Compare just the basename
            let base = comm.rsplit('/').next().unwrap_or(comm);
            if base == target {
                if kill_process_by_pid(pid) {
                    any_killed = true;
                }
            }
        }

        any_killed
    }
}
