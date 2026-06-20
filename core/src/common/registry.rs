use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use futures_util::future::join_all;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::common::{traits::Reporter, types::InstallStage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub name: String,
    pub description: Option<String>,
    pub shurikens: Vec<ArmoryItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ArmoryItem {
    #[serde(rename = "shuriken")]
    Shuriken {
        name: String,
        version: String,
        description: String,
        author: String,
        license: String,
        platforms: Vec<String>,
        url: String,
    },

    #[serde(rename = "bundle")]
    Bundle {
        name: String,
        version: String,
        description: String,
        author: String,
        license: String,
        shurikens: Vec<String>,
    },
}

impl ArmoryItem {
    pub fn name(&self) -> &str {
        match self {
            ArmoryItem::Shuriken { name, .. } => name,
            ArmoryItem::Bundle { name, .. } => name,
        }
    }

    pub fn is_shuriken(&self) -> bool {
        matches!(self, ArmoryItem::Shuriken { .. })
    }

    pub fn resolve(self) -> Self {
        match self {
            ArmoryItem::Shuriken {
                name,
                version,
                description,
                author,
                license,
                platforms,
                url,
            } => ArmoryItem::Shuriken {
                name,
                version,
                description,
                author,
                license,
                platforms,
                url: resolve_platform_placeholders(&url),
            },

            ArmoryItem::Bundle { .. } => self,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegistrySources {
    registries: HashMap<String, String>,
    client: reqwest::Client,
}

impl RegistrySources {
    pub fn new(registries: HashMap<String, String>) -> Self {
        Self {
            registries,
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_all(&self) -> HashMap<String, Registry> {
        let requests = self.registries.iter().map(|(name, url)| async {
            let result = fetch_registry_with_client(&self.client, url).await;
            (name.clone(), result)
        });

        join_all(requests)
            .await
            .into_iter()
            .filter_map(|(name, result)| match result {
                Ok(registry) => Some((name, registry)),
                Err(error) => {
                    eprintln!("Failed to fetch registry '{}': {}", name, error);
                    None
                }
            })
            .collect()
    }

    pub async fn all_shurikens(&self) -> Vec<ArmoryItem> {
        self.fetch_all()
            .await
            .into_values()
            .flat_map(|registry| registry.shurikens)
            .filter(ArmoryItem::is_shuriken)
            .collect()
    }

    pub async fn find_item(&self, registry_name: &str, item_name: &str) -> Result<ArmoryItem> {
        let registry_url = self.registry_url(registry_name)?;
        let registry = fetch_registry_with_client(&self.client, registry_url).await?;

        registry
            .shurikens
            .into_iter()
            .find(|item| item.name() == item_name)
            .with_context(|| {
                format!(
                    "Shuriken '{}' not found in registry '{}'",
                    item_name, registry_name
                )
            })
    }

    pub async fn find_shuriken_anywhere(&self, name: &str) -> Option<ArmoryItem> {
        self.all_shurikens()
            .await
            .into_iter()
            .find(|item| item.name() == name)
    }

    pub async fn download_url(&self, registry_name: &str, shuriken_name: &str) -> Result<String> {
        let registry_url = self.registry_url(registry_name)?.to_string();
        info!("Getting shuriken download URL from URL: {}", &registry_url);
        let item = self
            .find_item(registry_name, shuriken_name)
            .await?
            .resolve();

        match item {
            ArmoryItem::Shuriken { url, .. } => resolve_shuriken_url(&registry_url, &url),
            ArmoryItem::Bundle { .. } => {
                Err(anyhow::anyhow!("Bundles do not have direct download URLs"))
            }
        }
    }

    fn registry_url(&self, registry_name: &str) -> Result<&str> {
        self.registries
            .get(registry_name)
            .map(String::as_str)
            .with_context(|| format!("Registry {} not found in config.", registry_name))
    }
}

pub fn is_absolute_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub fn resolve_shuriken_url(registry_url: &str, shuriken_url: &str) -> Result<String> {
    if is_absolute_url(shuriken_url) {
        return Ok(resolve_platform_placeholders(shuriken_url));
    }

    let base = Url::parse(registry_url)?;
    let resolved = base.join(shuriken_url)?;

    Ok(resolve_platform_placeholders(resolved.as_str()))
}

fn resolve_platform_placeholders(value: &str) -> String {
    value
        .replace("{{ os }}", std::env::consts::OS)
        .replace("{{os}}", std::env::consts::OS)
        .replace("{{ arch }}", std::env::consts::ARCH)
        .replace("{{arch}}", std::env::consts::ARCH)
}

pub async fn fetch_registry(url: &str) -> Result<Registry> {
    let client = reqwest::Client::new();
    fetch_registry_with_client(&client, url).await
}

async fn fetch_registry_with_client(client: &reqwest::Client, url: &str) -> Result<Registry> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch registry '{}'", url))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read registry response '{}'", url))?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch registry '{}': HTTP {}",
            url,
            status
        ));
    }

    serde_yaml::from_str(&text).with_context(|| format!("Failed to parse registry '{}'", url))
}

pub async fn download_shuriken<R>(path: &Path, url: &str, tx: &R) -> Result<(), anyhow::Error>
where
    R: Reporter + Send + Sync + 'static,
{
    let response = reqwest::get(url).await?;

    let total_size = response.content_length().unwrap_or(0);

    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(path).await?;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;

    tx.stage(InstallStage::Downloading)?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        file.write_all(&chunk).await?;

        downloaded = downloaded.saturating_add(chunk.len() as u64);

        if total_size > 0 {
            let percent = (downloaded.saturating_mul(100) / total_size).min(100) as u8;

            tx.progress(percent)?;
        }
    }

    file.flush().await?;

    Ok(())
}

pub async fn get_shurikens_from_registries(urls: &[String]) -> Vec<ArmoryItem> {
    let registries = urls
        .iter()
        .enumerate()
        .map(|(index, url)| (index.to_string(), url.clone()))
        .collect();

    RegistrySources::new(registries).all_shurikens().await
}

pub async fn get_shuriken_from_registries(
    name: String,
    registries: &[String],
) -> Option<ArmoryItem> {
    let registries = registries
        .iter()
        .enumerate()
        .map(|(index, url)| (index.to_string(), url.clone()))
        .collect();

    RegistrySources::new(registries)
        .find_shuriken_anywhere(&name)
        .await
}
