use crate::{
    common::{
        config::NinjaConfig,
        registry::{ArmoryItem, download_shuriken, get_shurikens_from_registries},
        types::{ArmoryMetadata, FieldValue, ShurikenState},
    },
    scripting::{NinjaEngine, dsl::DslContext},
    shuriken::{Shuriken, ShurikenConfig},
    utils::{create_tar_gz_bytes, load_shurikens, normalize_shuriken_name},
};
use anyhow::{Context, Error, Result};
use dirs_next as dirs;
use either::Either::{self, Left, Right};
use log::{debug, info, warn};
use serde_cbor::to_vec;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
    str,
    sync::Arc,
};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    sync::RwLock,
};

const MAGIC: &[u8; 6] = b"HSRZEG";

/// A thin wrapper around a spawned process. We keep it simple: the
/// ManagedProcess owns a `tokio::process::Child` and provides async helpers.

#[derive(Clone, Debug)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub engine: Arc<Mutex<NinjaEngine>>,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub states: Arc<RwLock<HashMap<String, ShurikenState>>>,
    pub config: Arc<RwLock<crate::common::config::NinjaConfig>>,
}

impl ShurikenManager {
    pub async fn new() -> Result<Self> {
        let exe_dir = dirs::home_dir()
            .ok_or_else(|| Error::msg("Could not find home directory"))?
            .join(".ninja");

        fs::create_dir_all(&exe_dir).await?;

        let shurikens_dir = exe_dir.join("shurikens");
        let projects_dir = exe_dir.join("projects");

        // Create shurikens directory if it doesn't exist
        if !shurikens_dir.exists() {
            fs::create_dir(&shurikens_dir).await?;
        }

        if !projects_dir.exists() {
            fs::create_dir(&projects_dir).await?;
        }

        let (shurikens, states) = load_shurikens(&exe_dir).await?;

        let engine = NinjaEngine::new()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

        let config = if exe_dir.join("config.toml").exists() {
            let content = fs::read_to_string(exe_dir.join("config.toml")).await?;
            let config: NinjaConfig = toml::from_str(content.as_str())
                .map_err(|e| Error::msg(format!("Failed to parse config.toml: {}", e)))?;
            Arc::new(RwLock::new(config))
        } else {
            Arc::new(RwLock::new(NinjaConfig::new()))
        };

        Ok(Self {
            root_path: exe_dir,
            engine: Arc::new(Mutex::new(engine)),
            shurikens: Arc::new(RwLock::new(shurikens)),
            states: Arc::new(RwLock::new(states)),
            config,
        })
    }

    async fn update_state(&self, name: &str, new_state: ShurikenState) {
        let mut state_map = self.states.write().await;
        state_map.insert(name.to_string(), new_state);
    }

    pub async fn start(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens
            .get(&normalized_name)
            .ok_or_else(|| anyhow::Error::msg(format!("No such shuriken: {}", name)))?
            .clone();
        drop(shurikens);

        let shuriken_dir = self.root_path.join("shurikens").join(&normalized_name);
        if !shuriken_dir.exists() {
            return Err(anyhow::Error::msg(format!(
                "Shuriken directory not found: {}",
                shuriken_dir.display()
            )));
        }

        if let Err(e) = shuriken
            .start(
                &*self.engine.lock().await,
                &shuriken_dir,
                Some(self.clone()),
            )
            .await
        {
            return Err(anyhow::Error::msg(format!(
                "Failed to start shuriken '{}': {}",
                name, e
            )));
        }

        self.update_state(&normalized_name, ShurikenState::Running)
            .await;
        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        let (new_shurikens, new_states) = load_shurikens(&self.root_path).await?;
        *self.shurikens.write().await = new_shurikens;
        *self.states.write().await = new_states;
        debug!("Shuriken manager refreshed.");
        Ok(())
    }

    pub async fn configure(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(&normalized_name);

        if let Some(shuriken) = shuriken {
            let path = &self.root_path;
            shuriken.configure(path).await?;
        }
        Ok(())
    }

    pub async fn lockpick(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(&normalized_name);

        if let Some(shuriken) = shuriken {
            let path = &self.root_path;
            shuriken.lockpick(path).await?;
        }
        Ok(())
    }

    pub async fn save_config(&self, name: &str, data: HashMap<String, FieldValue>) -> Result<()> {
        println!("{:#?}", data);
        let normalized_name = normalize_shuriken_name(name);

        // Update in-memory config
        {
            let mut shurikens = self.shurikens.write().await;
            if let Some(shuriken) = shurikens.get_mut(&normalized_name) {
                if let Some(config) = &mut shuriken.config {
                    config.options = Some(data.clone());
                } else {
                    shuriken.config = Some(ShurikenConfig {
                        config_path: PathBuf::from("options.toml"),
                        options: Some(data.clone()),
                    });
                }
            }
        }

        // Write to disk
        let serialized_data = toml::ser::to_string_pretty(&data)?;
        let options_path = self
            .root_path
            .join("shurikens")
            .join(&normalized_name)
            .join(".ninja")
            .join("options.toml");

        // Ensure the parent directory exists
        if let Some(parent) = options_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Remove old file if it exists
        if options_path.exists() {
            fs::remove_file(&options_path).await?;
        }

        fs::write(&options_path, serialized_data).await?;

        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens
            .get(&normalized_name)
            .ok_or_else(|| anyhow::Error::msg(format!("No such shuriken: {}", name)))?
            .clone();
        drop(shurikens);

        let shuriken_dir = self.root_path.join("shurikens").join(&normalized_name);
        if !shuriken_dir.exists() {
            return Err(anyhow::Error::msg(format!(
                "Shuriken directory not found: {}",
                shuriken_dir.display()
            )));
        }

        if let Err(e) = shuriken
            .stop(
                &*self.engine.lock().await,
                &shuriken_dir,
                Some(self.clone()),
            )
            .await
        {
            return Err(anyhow::Error::msg(format!(
                "Failed to stop shuriken '{}': {}",
                name, e
            )));
        }

        self.update_state(&normalized_name, ShurikenState::Idle)
            .await;
        Ok(())
    }

    pub async fn get(&self, name: String) -> Result<Shuriken> {
        let partial_shuriken = &self.shurikens.read().await;
        let maybe_shuriken = partial_shuriken.get(&name);
        if let Some(shuriken) = maybe_shuriken {
            Ok(shuriken.clone())
        } else {
            Err(anyhow::Error::msg(format!(
                "No shuriken of name {} found",
                name
            )))
        }
    }

    pub async fn list(
        &self,
        state: bool,
    ) -> Result<Either<Vec<(String, ShurikenState)>, Vec<String>>> {
        if state {
            let states = self.states.read().await;
            let values: Vec<(String, ShurikenState)> =
                states.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Ok(Left(values))
        } else {
            let shurikens = self.shurikens.read().await;
            let keys: Vec<String> = shurikens.keys().cloned().collect();
            Ok(Right(keys))
        }
    }

    pub fn dsl_ctx(&self) -> DslContext {
        DslContext {
            selected: Arc::new(RwLock::new(None)),
            manager: self.clone(),
        }
    }

    pub async fn forge(
        &self,
        meta: ArmoryMetadata,
        path: PathBuf,
        output: Option<PathBuf>,
    ) -> Result<()> {
        let output = output.unwrap_or_else(|| self.root_path.join("blacksmith"));
        if !output.exists() {
            fs::create_dir_all(&output).await?;
        }
        let path = &self.root_path.join("shurikens").join(path);

        let shuriken_path = output.join(format!("{}-{}.shuriken", meta.id, meta.platform));
        let mut file = File::create(shuriken_path).await?;

        // ---- 1) Serialize metadata ----

        let serialized_metadata = to_vec(&meta)?;

        if serialized_metadata.len() > u16::MAX as usize {
            return Err(anyhow::Error::msg(
                "Metadata too large to fit in u16 length field",
            ));
        }

        // ---- 2) Build archive bytes (tar.gz) in a blocking thread ----
        let archive = {
            let path_clone = path.clone();
            tokio::task::spawn_blocking(move || create_tar_gz_bytes(&path_clone)).await??
        };
        let archive_len = archive.len();

        if archive_len > u32::MAX as usize {
            return Err(anyhow::Error::msg(
                "Archive too large to fit in u32 length field",
            ));
        }

        // ---- 3) Compute signature = SHA256(archive) ----
        let mut hasher = Sha256::new();
        
        hasher.update(&archive);
        let signature = hasher.finalize(); // 32 bytes

        // ---- 4) Write in correct order ----
        // [MAGIC]                 // 4 bytes
        // [metadata_length]       // u16 LE
        // [metadata]              // CBOR
        // [archive_length]        // u32 LE
        // [archive]               // tar.gz
        // [signature]             // 32 bytes SHA-256(archive)

        // MAGIC
        file.write_all(MAGIC).await?;

        // metadata_length (u16 LE)
        let meta_len_le = (serialized_metadata.len() as u16).to_le_bytes();
        file.write_all(&meta_len_le).await?;

        // metadata
        file.write_all(&serialized_metadata).await?;

        // archive_length (u32 LE)
        let archive_len_le = (archive_len as u32).to_le_bytes();
        file.write_all(&archive_len_le).await?;

        // archive
        file.write_all(&archive).await?;

        // signature
        file.write_all(&signature).await?;

        Ok(())
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        warn!("Deleting {}.", name);
        fs::remove_dir_all(format!("shurikens/{}", normalized_name)).await?;
        let _ = &self.shurikens.write().await.remove(&normalized_name);
        info!("Successfully deleted shuriken {}, refreshing.", name);
        #[cfg(debug_assertions)]
        dbg!("{:#?}", &self.shurikens);
        Ok(())
    }

    pub async fn reset_engine(&self) -> Result<()> {
        let new_engine = NinjaEngine::new()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        *self.engine.lock().await = new_engine; // don't ask i need to reset the engine everytime i run scripts in gui.
        Ok(())
    }

    // -------------------- Installation functions --------------------

    pub async fn install(&self, path: &Path) -> Result<()> {
        self.install_file(path).await
    }

    pub async fn install_url(&self, url: &str) -> Result<()> {
        let temp_path = self.root_path.join("temp_shuriken.shuriken");
        download_shuriken(&temp_path, url).await?;
        let result = self.install_file(&temp_path).await;
        let _ = fs::remove_file(temp_path).await; // clean up temp file
        result
    }

    /// Install a shuriken from a registry reference (e.g., "official:my-shuriken")
    pub async fn install_from_registry(
        &self,
        reference: &crate::common::config::ShurikenReference,
    ) -> Result<()> {
        let registries = &self.config.read().await.registries;
        let download_url =
            crate::common::config::resolve_download_url(registries, reference).await?;
        info!(
            "Installing shuriken {} from {}",
            reference.shuriken, download_url
        );
        self.install_url(&download_url).await
    }

    pub async fn install_file(&self, path: &Path) -> Result<(), anyhow::Error> {
        use sha2::{Digest, Sha256};
        use std::io::Cursor;

        info!("Starting installation");
        if !path.exists() {
            return Err(anyhow::Error::msg("Path does not exist"));
        }

        let mut file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| io::Error::other(format!("Failed to open shuriken file: {e}")))?;

        // 1) MAGIC (6 bytes)
        let mut magic_buf = [0u8; 6];
        file.read_exact(&mut magic_buf).await?;
        if &magic_buf != MAGIC {
            return Err(anyhow::Error::msg("Invalid shuriken file (bad MAGIC)."));
        }
        // this comment is just a small change
        // 2) metadata_length (u16 LE)
        let mut meta_len_buf = [0u8; 2];
        file.read_exact(&mut meta_len_buf).await?;
        let metadata_length = u16::from_le_bytes(meta_len_buf) as usize;

        const MAX_METADATA: usize = 64 * 1024; // 64 KB
        if metadata_length > MAX_METADATA {
            return Err(anyhow::Error::msg("Metadata too large"));
        }

        // 3) metadata (CBOR)
        let mut metadata_buf = vec![0u8; metadata_length];
        file.read_exact(&mut metadata_buf).await?;
        let metadata: ArmoryMetadata =
            serde_cbor::from_slice(&metadata_buf).context("Failed to parse metadata CBOR")?;

        info!("Metadata parsing complete");

        debug!("MAGIC:     {:?}", magic_buf);
        debug!("meta_len:  {}", metadata_length);
        debug!("metadata:  {:#?}", metadata);

        // 4) archive_length (u32 LE)
        let mut archive_len_buf = [0u8; 4];
        file.read_exact(&mut archive_len_buf).await?;
        let archive_length = u32::from_le_bytes(archive_len_buf) as usize;

        const MAX_ARCHIVE: usize = 1024 * 1024 * 1024; // 1 GB
        if archive_length > MAX_ARCHIVE {
            return Err(anyhow::Error::msg("Archive too large"));
        }

        // 5) archive (exactly archive_length bytes)
        let mut archive_buf = vec![0u8; archive_length];
        file.read_exact(&mut archive_buf).await?;

        // 6) signature (rest of the file)
        let mut signature = [0u8; 32];
        file.read_exact(&mut signature).await?;

        // Verify checksum = SHA256(MAGIC + metadata_len + metadata + archive_len + archive)
        let mut hasher = Sha256::new();
        hasher.update(&archive_buf);
        let digest = hasher.finalize();

        if digest.as_slice() != signature {
            return Err(anyhow::Error::msg(
                "Shuriken file signature mismatch (archive corrupted or tampered).",
            ));
        }

        // Platform check
        if !metadata.platform.contains(env::consts::OS)
            && !metadata.platform.contains(env::consts::ARCH)
        {
            return Err(anyhow::Error::msg(
                "Unsupported platform. Current platform is not the same as the shuriken's destined platform.",
            ));
        }

        // Unpack archive in blocking task
        let archive_cursor = Cursor::new(archive_buf);
        let archive_name = normalize_shuriken_name(&metadata.name);
        let unpack_path = self.root_path.clone().join("shurikens").join(&archive_name);
        let root_path = self.root_path.clone().join("shurikens").join(&archive_name);

        tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
            let gz_decoder = flate2::read::GzDecoder::new(archive_cursor);
            let mut archive = tar::Archive::new(gz_decoder);
            archive.unpack(&unpack_path)?;
            Ok(())
        })
        .await??;

        // Run postinstall script if present
        if let Some(pi_script) = &metadata.postinstall {
            info!("Running postinstall script");
            let engine = &self.engine.lock().await;
            engine.execute_file(pi_script, Some(&root_path), Some(self.clone()))?;
        }

        Ok(())
    }

    pub async fn registry_get_all_shurikens(&self) -> Vec<ArmoryItem> {
        let registries: Vec<String> = self
            .config
            .read()
            .await
            .registries
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let shurikens = get_shurikens_from_registries(&registries).await;
        shurikens
    }

    // -------------------- Project management API --------------------

    pub async fn get_projects(&self) -> Result<Vec<String>> {
        let path = &self.root_path.join("projects");
        let mut entries: Vec<String> = Vec::new();
        let mut fs_entries = fs::read_dir(path).await?;
        while let Some(entry) = fs_entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                if name == "pma" || name == "fancy-index" {
                    continue;
                }

                entries.push(name.to_string());
            }
        }
        Ok(entries)
    }
}
