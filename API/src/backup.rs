use std::io;
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum BackupFrequency {
    Daily,
    Weekly,
    Monthly,
}

pub fn install_backup_schedule(frequency: BackupFrequency) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let (schedule, modifier) = match frequency {
            BackupFrequency::Daily => ("DAILY", "/SC DAILY /ST 03:00"),
            BackupFrequency::Weekly => ("WEEKLY", "/SC WEEKLY /D MON /ST 03:00"),
            BackupFrequency::Monthly => ("MONTHLY", "/SC MONTHLY /D 1 /ST 03:00"),
        };

        Command::new("schtasks")
            .args(&[
                "/Create",
                "/TN",
                "NinjaBackup",
                "/TR",
                "ninja backup",
                "/SC",
                schedule,
                "/ST",
                "03:00", // 3 AM
            ])
            .status()?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Build cron expression
        let cron_expr = match frequency {
            BackupFrequency::Daily => "0 3 * * *",
            BackupFrequency::Weekly => "0 3 * * 1",
            BackupFrequency::Monthly => "0 3 1 * *",
        };

        // Append to user crontab
        let job = format!("{} /usr/local/bin/ninja backup\n", cron_expr);

        let output = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "(crontab -l 2>/dev/null; echo \"{}\") | crontab -",
                job.replace("\"", "\\\"")
            ))
            .output()?;

        if !output.status.success() {
            eprintln!("Failed to install cron job: {:?}", output);
        }
    }

    Ok(())
}

pub fn uninstall_backup_schedule() -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("schtasks")
            .args(&["/Delete", "/TN", "NinjaBackup", "/F"])
            .status()?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Filter out Ninja job from crontab
        let output = Command::new("bash")
            .arg("-c")
            .arg("crontab -l 2>/dev/null | grep -v 'ninja backup' | crontab -")
            .output()?;

        if !output.status.success() {
            eprintln!("Failed to remove cron job: {:?}", output);
        }
    }

    Ok(())
}
