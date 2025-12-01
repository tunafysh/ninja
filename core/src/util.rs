use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

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

pub fn make_admin_command<S: AsRef<str>>(bin: S, args: Option<&[String]>) -> std::io::Result<()> {
    let bin = bin.as_ref();

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(bin);
        if let Some(a) = args { cmd.args(a); }
        let status = cmd.status()?;
        if !status.success() {
            // fallback to sudo if pkexec not available
            let mut sudo = Command::new("sudo");
            sudo.arg(bin);
            if let Some(a) = args { sudo.args(a); }
            sudo.status()?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        use shell_escape::escape;
        let escaped_bin = escape(bin.into());
        let escaped_args = args
            .map(|a| a.iter().map(|x| escape(x.into()).to_string()).collect::<Vec<_>>().join(" "))
            .unwrap_or_default();

        let script = format!(
            "do shell script \"{} {}\" with administrator privileges",
            escaped_bin,
            escaped_args
        );

        let mut cmd = Command::new("osascript");
        cmd.arg("-e").arg(script).status()?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::UI::Shell::{ShellExecuteW, SEE_MASK_NO_CONSOLE};
        use windows::Win32::Foundation::{HWND, SW_HIDE};

        // Convert to UTF-16
        fn to_utf16(s: &str) -> Vec<u16> {
            OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
        }

        let lp_file = to_utf16(bin);

        let args_joined = args
            .map(|a| a.join(" "))
            .unwrap_or_default();
        let lp_parameters = to_utf16(&args_joined);

        unsafe {
            let result = ShellExecuteW(
                0 as HWND,                                           // hwnd
                to_utf16("runas").as_ptr(),                          // elevation verb
                lp_file.as_ptr(),                                    // file (exe)
                lp_parameters.as_ptr(),                              // arguments
                std::ptr::null(),                                    // working dir
                SW_HIDE,                                             // hide window
            );

            // ShellExecute returns >32 on success
            if result <= 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "ShellExecuteW failed to elevate",
                ));
            }
        }

        return Ok(());
    }
}
