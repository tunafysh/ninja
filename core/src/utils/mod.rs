pub mod download;

use crate::{
    common::types::{FieldValue, ShurikenState},
    shuriken::{Shuriken, ShurikenConfig},
};
use anyhow::{Error, Result, anyhow};
use flate2::{Compression, write::GzEncoder};
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tar::Builder as TarBuilder;
use tokio::{fs as async_fs, sync::Mutex};

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
        let query_handle = match OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return false,
        };

        if query_handle.is_invalid() {
            return false;
        }

        let _ = CloseHandle(query_handle);

        let terminate_handle = match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(h) => h,
            Err(_) => return false,
        };

        if terminate_handle.is_invalid() {
            return false;
        }

        let result = TerminateProcess(terminate_handle, 1).is_ok();
        let _ = CloseHandle(terminate_handle);
        result
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
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());

                let exe_os: OsString = OsString::from_wide(&entry.szExeFile[..len]);
                let exe = exe_os.to_string_lossy().to_string();

                if exe.to_ascii_lowercase() == target {
                    if kill_process_by_pid(entry.th32ProcessID) {
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
    use std::thread;
    use std::time::Duration;

    let pid = Pid::from_raw(pid as i32);

    match kill(pid, None) {
        Ok(_) => {
            if kill(pid, Signal::SIGTERM).is_ok() {
                for _ in 0..10 {
                    thread::sleep(Duration::from_millis(100));
                    if kill(pid, None).is_err() {
                        return true;
                    }
                }
            }

            kill(pid, Signal::SIGKILL).is_ok()
        }
        Err(Errno::ESRCH) => false,
        Err(Errno::EPERM) => kill(pid, Signal::SIGKILL).is_ok(),
        Err(_) => false,
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

/// Normalizes a shuriken name to lowercase for consistent directory naming.
/// This ensures all shuriken directories use lowercase names.
pub fn normalize_shuriken_name(name: &str) -> String {
    name.to_lowercase()
}

pub fn create_tar_gz_bytes(src_dir: &Path) -> Result<Vec<u8>> {
    if !src_dir.is_dir() {
        return Err(anyhow::Error::msg(format!(
            "Source directory does not exist or is not a directory: {}",
            src_dir.display()
        )));
    }

    let mut buf = Vec::new();

    {
        // Gzip wraps the in-memory buffer
        let enc = GzEncoder::new(&mut buf, Compression::default());
        let mut tar = TarBuilder::new(enc);

        // This recursively adds `src_dir` contents under "." in the archive
        tar.append_dir_all(".", src_dir)?;

        // Finish tar, then finish gzip
        let enc = tar.into_inner()?; // GzEncoder
        enc.finish()?; // flush into buf
    }

    Ok(buf)
}

// Shared logic for loading shurikens from disk
pub async fn load_shurikens(root_path: &Path) -> Result<HashMap<String, Shuriken>> {
    let shurikens_dir = root_path.join("shurikens");
    let mut shurikens = HashMap::new();

    // Only iterate immediate children of `shurikens/`
    let mut dir = match async_fs::read_dir(&shurikens_dir).await {
        Ok(d) => d,
        Err(_) => return Ok(shurikens), // no shurikens dir = empty
    };

    while let Some(entry) = dir.next_entry().await? {
        let shuriken_path = entry.path();
        if !shuriken_path.is_dir() {
            continue;
        }

        let name = match shuriken_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_owned(),
            None => continue, // skip non-UTF8 names
        };

        let ninja_dir = shuriken_path.join(".ninja");

        // 1. Load manifest (required)
        let manifest_path = ninja_dir.join("manifest.toml");
        if !manifest_path.exists() {
            continue; // not a valid shuriken
        }

        let content: String = async_fs::read_to_string(&manifest_path).await?;

        let mut shuriken: Shuriken = toml::from_str(&content)
            .map_err(|e| Error::msg(format!("TOML error in {}: {}", manifest_path.display(), e)))?;

        // 2. Check for lock file
        let lock_path = ninja_dir.join("shuriken.lck");
        let state = if lock_path.exists() {
            ShurikenState::Running
        } else {
            ShurikenState::Idle
        };

        shuriken.state = Arc::new(Mutex::new(state.clone()));

        // 3. Load options (optional)
        let options_path = ninja_dir.join("options.toml");
        if options_path.exists() {
            let content: String = async_fs::read_to_string(&options_path).await?;
            let options: HashMap<String, FieldValue> = toml::from_str(&content).map_err(|e| {
                Error::msg(format!(
                    "Options error in {}: {}",
                    options_path.display(),
                    e
                ))
            })?;

            if let Some(config) = &mut shuriken.config {
                config.options = Some(options);
            } else {
                shuriken.config = Some(ShurikenConfig {
                    config_path: PathBuf::from("options.toml"),
                    options: Some(options),
                });
            }
        }

        // Store using the directory name (which should already be lowercase)
        // but normalize it to be sure
        let normalized_name = normalize_shuriken_name(&name);
        shurikens.insert(normalized_name.clone(), shuriken);
    }

    Ok(shurikens)
}

pub struct PortOwner {
    pub pid: u32,
    pub name: Option<String>,
}

#[cfg(target_os = "linux")]
pub fn get_port_owner(port: u16) -> Option<PortOwner> {
    use std::fs;

    let content = fs::read_to_string("/proc/net/tcp").ok()?;

    let port_hex = format!("{:04X}", port);

    for line in content.lines().skip(1) {
        let cols: Vec<_> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }

        let local_address = cols[1]; // IP:PORT in hex
        if local_address.ends_with(&port_hex) {
            let inode = cols[9];

            // map inode → pid via /proc/*/fd
            if let Some(pid) = find_pid_by_inode(inode) {
                return Some(PortOwner { pid, name: None });
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn find_pid_by_inode(inode: &str) -> Option<u32> {
    use std::fs;

    for entry in fs::read_dir("/proc").ok()? {
        let entry = entry.ok()?;
        let pid_str = entry.file_name().to_string_lossy().to_string();

        if !pid_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let fd_path = format!("/proc/{pid_str}/fd");

        if let Ok(fds) = fs::read_dir(fd_path) {
            for fd in fds.flatten() {
                if let Ok(link) = fs::read_link(fd.path()) {
                    if link.to_string_lossy().contains(inode) {
                        return pid_str.parse().ok();
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
pub fn get_port_owner(port: u16) -> Option<PortOwner> {
    use std::process::Command;

    let output = Command::new("netstat")
        .args(["-anv", "-p", "tcp"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        if line.contains(&format!(".{}", port)) {
            // macOS does not reliably expose PID here without root
            return Some(PortOwner { pid: 0, name: None });
        }
    }

    None
}

#[cfg(target_os = "windows")]
pub fn get_port_owner(port: u16) -> Option<PortOwner> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::Foundation::*;

    unsafe {
        let mut size = 0u32;

        GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            false.into(),
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );

        let mut buffer = vec![0u8; size as usize];

        let res = GetExtendedTcpTable(
            buffer.as_mut_ptr() as *mut _,
            &mut size,
            false.into(),
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );

        if res != NO_ERROR.0 as i32 {
            return None;
        }

        let table = buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
        let table = &*table;

        let rows = std::slice::from_raw_parts(
            table.table.as_ptr(),
            table.dwNumEntries as usize,
        );

        for row in rows {
            let local_port = u16::from_be((*row).dwLocalPort as u16);

            if local_port == port {
                return Some(PortOwner {
                    pid: row.dwOwningPid,
                    name: None,
                });
            }
        }
    }

    None
}