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

pub fn make_admin_command(bin: &str, args: Option<&[String]>) -> Command {
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
        cmd.arg("-WindowStyle");
        cmd.arg("Hidden");
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