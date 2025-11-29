use std::process::Command;
use crate::manager::ShurikenManager;
use ignore::WalkBuilder;
use opendal::Operator;
use opendal::services::Fs;
use std::fs::File;
use tar::Builder as TarBuilder;
use flate2::write::GzEncoder;
use flate2::Compression;
use chrono::Utc;

#[derive(Debug, Clone, Copy)]
pub enum BackupFrequency {
    Daily,
    Weekly,
    Monthly,
}

pub fn install_backup_schedule(frequency: BackupFrequency) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let (schedule, _) = match frequency {
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
                "03:00",
            ])
            .status()?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let cron_expr = match frequency {
            BackupFrequency::Daily => "0 3 * * *",
            BackupFrequency::Weekly => "0 3 * * 1",
            BackupFrequency::Monthly => "0 3 1 * *",
        };

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

pub fn uninstall_backup_schedule() -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("schtasks")
            .args(&["/Delete", "/TN", "NinjaBackup", "/F"])
            .status()?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
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

pub async fn create_backup(manager: &ShurikenManager) -> Result<(), String> {
    let backup_dir = manager.root_path.join("backups");

    // Make sure backup directory exists (async)
    if !backup_dir.exists() {
        tokio::fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| e.to_string())?;
    }

    let backup_file_path = backup_dir.join(format!(
        "backup-{}.tar.gz",
        Utc::now().format("%Y-%m-%d-%H-%M-%S")
    ));

    // Run synchronous backup in blocking task
    let projects_path = manager.root_path.join("projects");
    let backup_file_path_clone = backup_file_path.clone();

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let backup_file = File::create(&backup_file_path_clone)
            .map_err(|e| e.to_string())?;
        let mut gzip = GzEncoder::new(backup_file, Compression::default());
        {
            
        let mut tar = TarBuilder::new(&mut gzip);

        for entry in WalkBuilder::new(&projects_path)
            .git_ignore(true)
            .git_global(true)
            .ignore(true)
            .build()
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                let path = entry.path();
                let rel_path = path
                    .strip_prefix(&projects_path)
                    .map_err(|e| e.to_string())?;
                tar.append_path_with_name(path, rel_path)
                    .map_err(|e| e.to_string())?;
            }
        }
        tar.finish().map_err(|e| e.to_string())?;
        }

        
        
        // Then finish gzip
        gzip.finish().map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Optional: upload to Opendal (async)
    let fs_builder = Fs::default().root(&backup_dir.display().to_string());
    let fs_op = Operator::new(fs_builder)
        .map_err(|e| e.to_string())?
        .finish();

    let backup_name = backup_file_path
        .file_name()
        .unwrap()
        .to_string_lossy();
    let data = tokio::fs::read(&backup_file_path)
        .await
        .map_err(|e| e.to_string())?;
    fs_op.write(&backup_name, data).await.map_err(|e| e.to_string())?;

    Ok(())
}
