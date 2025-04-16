use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use anyhow::{Result, Context};
use std::fs;

/// Global registry tracking all installed shurikens
#[derive(Debug, Serialize, Deserialize)]
pub struct ShurikenRegistry {
    #[serde(skip_serializing)]
    pub registry_path: PathBuf,
    pub shurikens: HashMap<String, ShurikenEntry>,
}

/// Registry entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShurikenEntry {
    pub config_path: PathBuf,
    pub installed_at: chrono::DateTime<chrono::Local>,
    pub enabled: bool,
    pub dependencies: Vec<String>,
    pub version: String,
}

/// Embedded registry metadata in component config
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub version: String,
    pub author: String,
    pub description: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShurikenConfig {
    pub name: String,
    pub entry_point: PathBuf,
    pub args: Vec<String>,
    pub config_path: PathBuf,
    pub working_dir: PathBuf,
    pub log_path: PathBuf,
    pub port: Option<u16>,
    pub env: Option<Vec<(String, String)>>,
    #[serde(rename = "registry")]
    pub registry_metadata: RegistryMetadata,
}

impl ShurikenConfig {
    /// Load configuration from TOML file with path resolution
    pub fn load(config_path: &Path) -> Result<Self>
    where
        Self: DeserializeOwned,
    {
        let config_content = std::fs::read_to_string(config_path)
            .context("Failed to read shuriken config file")?;

        let mut config: ShurikenConfig = toml::from_str(&config_content)
            .context("Failed to parse shuriken config")?;

        // Resolve relative paths based on config file location
        let base_path = config_path.parent()
            .unwrap_or_else(|| Path::new("."));

        config.entry_point = base_path.join(&config.entry_point);
        config.config_path = base_path.join(&config.config_path);
        config.working_dir = base_path.join(&config.working_dir);
        config.log_path = base_path.join(&config.log_path);

        Ok(config)
    }
}

impl ShurikenRegistry {
    /// Initialize or load existing registry
    pub fn new(registry_path: impl AsRef<Path>) -> Result<Self> {
        let path = registry_path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)
                .context("Failed to read registry file")?;
            let mut reg: ShurikenRegistry = toml::from_str(&content)
                .context("Invalid registry format")?;
            reg.registry_path = path.to_path_buf();
            Ok(reg)
        } else {
            // Create parent directories for registry file
            let parent = path.parent().context("Registry path has no parent directory")?;
            fs::create_dir_all(parent).context("Failed to create registry directory")?;
    
            // Create shurikens directory
            let shurikens_dir = parent.join("shurikens");
            fs::create_dir_all(&shurikens_dir).context("Failed to create shurikens directory")?;
    
            // Create new registry instance
            let reg = Self {
                registry_path: path.to_path_buf(),
                shurikens: HashMap::new(),
            };
    
            // Write initial registry file
            let toml_content = toml::to_string(&reg)
                .context("Failed to serialize registry")?;
            fs::write(path, toml_content)
                .context("Failed to write registry file")?;
    
            Ok(reg)
        }
    }

    /// Install a new shuriken
    pub fn install(&mut self, config_path: &Path) -> Result<()> {
        // Load and validate config first
        let config = ShurikenConfig::load(config_path)
            .context("Failed to load shuriken config")?;

        // Check for existing installation
        if self.shurikens.contains_key(&config.name) {
            return Err(anyhow::anyhow!(
                "Shuriken '{}' already installed (version {})",
                config.name,
                self.shurikens[&config.name].version
            ));
        }

        // Create registry entry
        let entry = ShurikenEntry {
            config_path: config_path.to_path_buf(),
            installed_at: chrono::Local::now(),
            enabled: true,
            dependencies: config.registry_metadata.dependencies.clone(),
            version: config.registry_metadata.version.clone(),
        };

        self.shurikens.insert(config.name.clone(), entry);
        self.save()
    }

    /// Uninstall a shuriken
    pub fn uninstall(&mut self, name: &str) -> Result<()> {
        self.shurikens.remove(name)
            .ok_or_else(|| anyhow::anyhow!("Shuriken not found"))?;
        self.save()
    }

    /// Enable/disable a shuriken
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        if let Some(entry) = self.shurikens.get_mut(name) {
            entry.enabled = enabled;
            self.save()
        } else {
            Err(anyhow::anyhow!("Shuriken not found"))
        }
    }

    /// Save registry to disk
    fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize registry")?;
        fs::write(&self.registry_path, content)
            .context("Failed to write registry file")
    }

    /// List installed shurikens
    pub fn list_installed(&self) -> Vec<&ShurikenEntry> {
        self.shurikens.values().collect()
    }

    /// Find shuriken by name
    pub fn find(&self, name: &str) -> Option<&ShurikenEntry> {
        self.shurikens.get(name)
    }
}