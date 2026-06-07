use std::path::Path;

use futures_util::future::join_all;
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::common::{traits::Reporter, types::InstallStage};

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

impl ArmoryItem {
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
            } => {
                let resolved_partial_url = url.replace("{{ os }}", std::env::consts::OS);
                let resolved_url =
                    resolved_partial_url.replace("{{ arch }}", std::env::consts::ARCH);

                ArmoryItem::Shuriken {
                    name,
                    version,
                    description,
                    author,
                    license,
                    platforms,
                    url: resolved_url,
                }
            }

            ArmoryItem::Bundle { .. } => self,
        }
    }
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

        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = (downloaded * 100 / total_size) as u8;

            tx.progress(percent)?;
        }
    }

    file.flush().await?;

    Ok(())
}

pub async fn get_shurikens_from_registries(urls: &[String]) -> Vec<ArmoryItem> {
    let futures = urls.iter().map(|url| fetch_registry(url));
    let results = join_all(futures).await;

    let mut all_shurikens = Vec::new();

    for result in results {
        match result {
            Ok(registry) => {
                let shurikens = registry
                    .shurikens
                    .into_iter()
                    .filter(|item| matches!(item, ArmoryItem::Shuriken { .. }));

                all_shurikens.extend(shurikens);
            }
            Err(e) => {
                eprintln!("Failed to fetch registry: {}", e);
            }
        }
    }

    all_shurikens
}

pub async fn get_shuriken_from_registries(
    name: String,
    registries: &[String],
) -> Option<ArmoryItem> {
    let shurikens = get_shurikens_from_registries(registries).await;

    shurikens.into_iter().find(|item| match item {
        ArmoryItem::Shuriken {
            name: item_name, ..
        } => item_name == &name,
        _ => false,
    })
}
