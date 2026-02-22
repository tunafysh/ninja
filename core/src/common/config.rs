use log::{self, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::common::registry::{ArmoryItem, is_absolute_url};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShurikenReference {
    pub registry: String,
    pub shuriken: String,
}

impl ShurikenReference {
    /// Parse a "registry:shuriken" format string
    pub fn parse(input: &str) -> Result<Self, anyhow::Error> {
        let parts: Vec<&str> = input.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid shuriken reference format. Expected 'registry:shuriken', got '{}'",
                input
            ));
        }

        Ok(ShurikenReference {
            registry: parts[0].to_string(),
            shuriken: parts[1].to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NinjaConfig {
    registries: HashMap<String, String>,
    check_updates: bool,
    dev_mode: bool,
}

impl NinjaConfig {
    pub fn new() -> Self {
        Self {
            registries: HashMap::from([(
                "ninja".to_string(),
                "https://raw.githubusercontent.com/tunafysh/ninja-registry/main/registry.yaml"
                    .to_string(),
            )]),
            check_updates: true,
            dev_mode: false,
        }
    }

    pub fn add_registry(&mut self, name: String, url: String) {
        self.registries.insert(name, url);
    }

    pub fn set_check_updates(&mut self, check: bool) {
        self.check_updates = check;
    }

    pub fn set_dev_mode(&mut self, dev_mode: bool) {
        self.dev_mode = dev_mode;
    }
    pub fn get_registries(&self) -> &HashMap<String, String> {
        &self.registries
    }
    pub fn remove_registry(&mut self, registry: &str) {
        self.registries.remove(registry);
    }
}

impl Default for NinjaConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn fetch_registries(
    config: &NinjaConfig,
) -> HashMap<String, crate::common::registry::Registry> {
    let mut registries = HashMap::new();

    for (name, url) in config.get_registries() {
        info!("Fetching registry '{}' from {}", name, url);
        match crate::common::registry::fetch_registry(url).await {
            Ok(reg) => {
                info!(
                    "Successfully fetched registry '{}' with {} items",
                    name,
                    reg.shurikens.len()
                );
                registries.insert(name.clone(), reg);
            }
            Err(e) => error!("Failed to fetch registry '{}': {}", name, e),
        }
    }

    registries
}

pub fn resolve_shuriken_url(
    registry_url: &str,
    shuriken_url: &str,
) -> Result<String, anyhow::Error> {
    if is_absolute_url(shuriken_url) {
        return Ok(shuriken_url.to_string());
    }

    let base = Url::parse(registry_url)?;
    let resolved = base.join(shuriken_url)?;
    Ok(resolved.into())
}

/// Find a shuriken in the fetched registries by its reference (e.g., "official:my-shuriken")
pub fn find_shuriken_in_registries(
    registries: &HashMap<String, crate::common::registry::Registry>,
    reference: &ShurikenReference,
) -> Result<(ArmoryItem, String), anyhow::Error> {
    let registry = registries
        .get(&reference.registry)
        .ok_or_else(|| anyhow::anyhow!("Registry '{}' not found", reference.registry))?;

    let shuriken = registry
        .shurikens
        .iter()
        .find(|s| match s {
            ArmoryItem::Shuriken { name, .. } => name == &reference.shuriken,
            ArmoryItem::Bundle { name, .. } => name == &reference.shuriken,
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Shuriken '{}' not found in registry '{}'",
                reference.shuriken,
                reference.registry
            )
        })?;

    Ok((shuriken.clone(), reference.registry.clone()))
}

/// Get information about a shuriken from registries as JSON
pub fn get_shuriken_info(
    registries: &HashMap<String, crate::common::registry::Registry>,
    reference: &ShurikenReference,
) -> Result<serde_json::Value, anyhow::Error> {
    let (shuriken, registry_name) = find_shuriken_in_registries(registries, reference)?;

    let info = match shuriken {
        ArmoryItem::Shuriken {
            name,
            version,
            description,
            author,
            license,
            platforms,
            url,
        } => {
            serde_json::json!({
                "type": "shuriken",
                "name": name,
                "version": version,
                "registry": registry_name,
                "author": author,
                "license": license,
                "description": description,
                "platforms": platforms,
                "url": url
            })
        }
        ArmoryItem::Bundle {
            name,
            version,
            description,
            author,
            license,
            shurikens,
        } => {
            serde_json::json!({
                "type": "bundle",
                "name": name,
                "version": version,
                "registry": registry_name,
                "author": author,
                "license": license,
                "description": description,
                "contains": shurikens
            })
        }
    };

    Ok(info)
}

/// Resolve the download URL for a shuriken from registries
pub fn resolve_download_url(
    registries: &HashMap<String, crate::common::registry::Registry>,
    reference: &ShurikenReference,
) -> Result<String, anyhow::Error> {
    let registry = registries
        .get(&reference.registry)
        .ok_or_else(|| anyhow::anyhow!("Registry '{}' not found", reference.registry))?;

    let registry_url = format!(
        "https://raw.githubusercontent.com/tunafysh/ninja-registry/main/{}.yaml",
        &reference.registry
    );

    let shuriken = registry
        .shurikens
        .iter()
        .find(|s| match s {
            ArmoryItem::Shuriken { name, .. } => name == &reference.shuriken,
            ArmoryItem::Bundle { name, .. } => name == &reference.shuriken,
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Shuriken '{}' not found in registry '{}'",
                reference.shuriken,
                reference.registry
            )
        })?;

    let shuriken_url = match shuriken {
        ArmoryItem::Shuriken { url, .. } => url,
        _ => return Err(anyhow::anyhow!("Bundles do not have direct download URLs")),
    };

    resolve_shuriken_url(&registry_url, shuriken_url)
}
