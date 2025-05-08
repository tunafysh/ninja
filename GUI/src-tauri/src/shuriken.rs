use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use glob::glob;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct ShurikenConfig {
    name: String,
    version: String,
    entry_point: PathBuf,
    args: Vec<String>,
    #[serde(skip)]
    base_dir: PathBuf,
    #[serde(skip)]
    process: Arc<Mutex<Option<Child>>>,
}

impl ShurikenConfig {
    pub fn load(config_path: &Path) -> Result<Self> {
        let config_content = std::fs::read_to_string(config_path)
            .context("Failed to read manifest file")?;

        let mut config: Self = toml::from_str(&config_content)
            .context("Failed to parse Shuriken manifest")?;

        // Set base directory relative to config file location
        config.base_dir = config_path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        // Validate essential paths
        let full_entry = config.absolute_path(&config.entry_point);
        if !full_entry.exists() {
            return Err(anyhow::anyhow!(
                "Entry point not found: {}",
                full_entry.display()
            ));
        }

        Ok(config)
    }

    pub fn absolute_path(&self, relative_path: &Path) -> PathBuf {
        self.base_dir.join(relative_path)
    }

    pub fn start(&mut self) -> Result<()> {
        let mut process = Command::new(self.absolute_path(&self.entry_point))
            .args(&self.args)
            .current_dir(&self.base_dir)
            .spawn()
            .context("Failed to start Shuriken")?;

        *self.process.lock().unwrap() = Some(process);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(process) = self.process.lock().unwrap().as_mut() {
            process.kill()?;
        }
        Ok(())
    }
}

pub struct ShurikenManager;

impl ShurikenManager {
    pub fn discover(pattern: &str) -> Result<Vec<ShurikenConfig>> {
        let mut shurikens = Vec::new();

        for entry in glob(pattern).context("Invalid glob pattern")? {
            match entry {
                Ok(path) => match ShurikenConfig::load(&path) {
                    Ok(config) => shurikens.push(config),
                    Err(e) => eprintln!("Skipping invalid Shuriken at {}: {}", path.display(), e),
                },
                Err(e) => eprintln!("Glob error: {}", e),
            }
        }

        Ok(shurikens)
    }

    pub fn find_by_name<'a>(shurikens: &'a [ShurikenConfig], name: &'a str) -> Option<&'a ShurikenConfig> {
        shurikens.iter().find(|s| s.name == name)
    }
}