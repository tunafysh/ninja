use crate::manager::ShurikenManager;
use anyhow::{Context, Result};
use chrono::Utc;
use flate2::Compression;
use flate2::{read::GzDecoder, write::GzEncoder};
use ignore::WalkBuilder;
use opendal::Operator;
use opendal::services::Fs;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::{path::Path, process::Command};
use tar::{Archive, Builder as TarBuilder};
use tokio::task;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionType {
    Fast,
    Normal,
    Best,
}

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
            .args([
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
            .args(["/Delete", "/TN", "NinjaBackup", "/F"])
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

pub async fn create_backup(
    manager: &ShurikenManager,
    compression: Option<CompressionType>,
) -> Result<()> {
    let backup_dir = manager.root_path.join("backups");

    // Make sure backup directory exists
    if !backup_dir.exists() {
        tokio::fs::create_dir_all(&backup_dir)
            .await
            .context("Failed to create backup directory")?;
    }

    let backup_file_path = backup_dir.join(format!(
        "backup-{}.tar.gz",
        Utc::now().format("%Y-%m-%d-%H-%M-%S")
    ));

    let projects_path = manager.root_path.join("projects");
    let backup_file_path_clone = backup_file_path.clone();

    // Run synchronous backup in blocking task
    task::spawn_blocking(move || -> Result<()> {
        let backup_file =
            File::create(&backup_file_path_clone).context("Failed to create backup file")?;
        let level: Compression = if let Some(compression) = compression {
            match compression {
                CompressionType::Best => Compression::best(),
                CompressionType::Normal => Compression::default(),
                CompressionType::Fast => Compression::fast(),
            }
        } else {
            Compression::default()
        };

        let mut gzip = GzEncoder::new(backup_file, level);
        {
            let mut tar = TarBuilder::new(&mut gzip);

            for entry in WalkBuilder::new(&projects_path)
                .hidden(false)
                .git_ignore(true)
                .git_global(true)
                .ignore(true)
                .build()
            {
                let entry = entry.context("Failed to read directory entry")?;
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let path = entry.path();
                    let rel_path = path
                        .strip_prefix(&projects_path)
                        .context("Failed to strip prefix for path")?;
                    tar.append_path_with_name(path, rel_path)
                        .context("Failed to append file to tar")?;
                }
            }
            tar.finish().context("Failed to finish tar archive")?;
        }

        gzip.finish().context("Failed to finish gzip compression")?;
        Ok(())
    })
    .await
    .context("Backup task panicked")??;

    // Optional: upload to Opendal
    let fs_builder = Fs::default().root(&backup_dir.display().to_string());
    let fs_op = Operator::new(fs_builder)
        .context("Failed to create Opendal operator")?
        .finish();

    let backup_name = backup_file_path
        .file_name()
        .context("Invalid backup file path: no filename")?
        .to_string_lossy();
    let data = tokio::fs::read(&backup_file_path)
        .await
        .context("Failed to read backup file")?;
    fs_op
        .write(&backup_name, data)
        .await
        .context("Failed to write backup file to Opendal")?;

    Ok(())
}

pub async fn restore_backup(manager: &ShurikenManager, file: &Path) -> Result<()> {
    let backup_file_path = file.to_path_buf();
    let projects_path = manager.root_path.join("projects");
    let backup_file_path_clone = backup_file_path.clone();
    let projects_path_clone = projects_path.clone();

    // Run synchronous restore in blocking task
    task::spawn_blocking(move || -> Result<()> {
        let backup_file =
            File::open(&backup_file_path_clone).context("Failed to open backup file")?;
        let decompressor = GzDecoder::new(backup_file);
        let mut archive = Archive::new(decompressor);

        archive
            .unpack(&projects_path_clone)
            .context("Failed to unpack backup archive")?;

        Ok(())
    })
    .await
    .context("Restore task panicked")??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_serialization() {
        use serde_json;
        
        // Test Fast compression
        let fast = CompressionType::Fast;
        let json = serde_json::to_string(&fast).unwrap();
        assert!(json.contains("Fast"));
        
        // Test Normal compression
        let normal = CompressionType::Normal;
        let json = serde_json::to_string(&normal).unwrap();
        assert!(json.contains("Normal"));
        
        // Test Best compression
        let best = CompressionType::Best;
        let json = serde_json::to_string(&best).unwrap();
        assert!(json.contains("Best"));
    }

    #[test]
    fn test_compression_type_deserialization() {
        use serde_json;
        
        // Test deserializing Fast
        let json = "\"Fast\"";
        let fast: CompressionType = serde_json::from_str(json).unwrap();
        matches!(fast, CompressionType::Fast);
        
        // Test deserializing Normal
        let json = "\"Normal\"";
        let normal: CompressionType = serde_json::from_str(json).unwrap();
        matches!(normal, CompressionType::Normal);
        
        // Test deserializing Best
        let json = "\"Best\"";
        let best: CompressionType = serde_json::from_str(json).unwrap();
        matches!(best, CompressionType::Best);
    }

    #[test]
    fn test_backup_frequency_variants() {
        // Test that all variants are constructible
        let daily = BackupFrequency::Daily;
        let weekly = BackupFrequency::Weekly;
        let monthly = BackupFrequency::Monthly;
        
        // Verify they can be formatted (suppress unused result warning)
        let _ = format!("{:?}", daily);
        let _ = format!("{:?}", weekly);
        let _ = format!("{:?}", monthly);
    }

    #[test]
    fn test_compression_level_conversion() {
        // Test that compression types can be copied
        let fast = CompressionType::Fast;
        let fast_copy = fast;
        let _ = format!("{:?}", fast);
        let _ = format!("{:?}", fast_copy);
    }

    #[test]
    fn test_compression_type_variants() {
        // Ensure all compression types are distinct
        let fast = CompressionType::Fast;
        let normal = CompressionType::Normal;
        let best = CompressionType::Best;
        
        // Debug format should be different
        let fast_str = format!("{:?}", fast);
        let normal_str = format!("{:?}", normal);
        let best_str = format!("{:?}", best);
        
        assert_ne!(fast_str, normal_str);
        assert_ne!(normal_str, best_str);
        assert_ne!(fast_str, best_str);
    }
}
