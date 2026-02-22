use std::path::Path;

use reqwest;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
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

pub fn is_absolute_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub async fn fetch_registry(url: &str) -> Result<Registry, anyhow::Error> {
    let response = reqwest::get(url).await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow::anyhow!("Failed to fetch registry: HTTP {}", status));
    }

    let registry: Registry = serde_yaml::from_str(&text)?;
    Ok(registry)
}

pub async fn download_shuriken(path: &Path, url: &str) -> Result<(), anyhow::Error> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?.to_vec();
    fs::write(path, &bytes).await?;
    Ok(())
}
