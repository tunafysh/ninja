use crate::{
    common::{
        config::{NinjaConfig, ShurikenReference},
        registry::{Registry, RegistrySources, download_shuriken},
        traits::Reporter,
        types::{ArmoryMetadata, FieldValue, InstallStage, ShurikenState},
    },
    scripting::{NinjaEngine, dsl::DslEngine},
    shuriken::{Shuriken, ShurikenConfig},
    utils::{create_tar_gz_bytes, load_shurikens, normalize_shuriken_name},
};
use anyhow::{Context, Error, Result};
use ciborium::{from_reader, ser::into_writer};
use dirs_next as dirs;
use either::Either::{self, Left, Right};
use flate2::read::GzDecoder;
use futures_util::future::join_all;
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env, io,
    marker::Send,
    path::{Path, PathBuf},
    str,
    sync::Arc,
};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, RwLock},
};

const MAGIC: &[u8; 6] = b"HSRZEG";

/// A thin wrapper around a spawned process. We keep it simple: the
/// ManagedProcess owns a `tokio::process::Child` and provides async helpers.

/// The main orchestrator for managing Shurikens and their lifecycle.
///
/// `ShurikenManager` handles all operations related to Shuriken services,
/// including startup, configuration, installation, and lifecycle management.
/// It maintains the scripting engine, configuration, and in-memory state.
///
/// # Fields
/// - `root_path`: Base directory where Ninja stores data (~/.ninja)
/// - `engine`: Lua scripting engine for executing Shuriken scripts
/// - `shurikens`: Cached map of loaded Shurikens by name
/// - `config`: Global Ninja configuration including registries
#[derive(Clone, Debug)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub engine: Arc<Mutex<NinjaEngine>>,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub config: Arc<RwLock<crate::common::config::NinjaConfig>>,
}

impl ShurikenManager {
    /// Creates a new `ShurikenManager` instance.
    ///
    /// Initializes the Ninja directory structure (~/.ninja), loads existing Shurikens,
    /// creates a Lua scripting engine, and loads or generates the global configuration.
    ///
    /// # Returns
    /// - `Ok(ShurikenManager)` on success
    /// - `Err` if home directory cannot be found or initialization fails
    ///
    /// # Panics
    /// None - all errors are returned as Results
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

        let shurikens = load_shurikens(&exe_dir).await?;

        let engine = NinjaEngine::new()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

        let config = if exe_dir.join("config.toml").exists() {
            let content = fs::read_to_string(exe_dir.join("config.toml")).await?;
            let config: NinjaConfig = toml::from_str(content.as_str())
                .map_err(|e| Error::msg(format!("Failed to parse config.toml: {}", e)))?;
            Arc::new(RwLock::new(config))
        } else {
            let config = NinjaConfig::new();
            config.generate_default_config(&exe_dir).await?;
            Arc::new(RwLock::new(config))
        };

        Ok(Self {
            root_path: exe_dir,
            engine: Arc::new(Mutex::new(engine)),
            shurikens: Arc::new(RwLock::new(shurikens)),
            config,
        })
    }

    /// Updates the state of a Shuriken (internal helper).
    ///
    /// # Arguments
    /// - `shuriken`: The Shuriken instance to update
    /// - `new_state`: The new state to set
    async fn update_state(&self, shuriken: Shuriken, new_state: ShurikenState) {
        let mut state_lock = shuriken.state.lock().await;
        *state_lock = new_state;
    }

    /// Starts a Shuriken by name.
    ///
    /// Executes the Shuriken's startup script and begins running the service.
    /// Updates the Shuriken's state to `Running` on success.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to start
    ///
    /// # Returns
    /// - `Ok(())` if startup completed successfully
    /// - `Err` if Shuriken not found, script execution fails, or startup errors occur
    pub async fn start(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        info!("Starting shuriken: {}", name);

        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens
            .get(&normalized_name)
            .ok_or_else(|| {
                warn!("Shuriken not found: {}", name);
                anyhow::Error::msg(format!("No such shuriken: {}", name))
            })?
            .clone();
        drop(shurikens);

        let shuriken_dir = self.root_path.join("shurikens").join(&normalized_name);
        if !shuriken_dir.exists() {
            warn!("Shuriken directory not found: {}", shuriken_dir.display());
            return Err(anyhow::Error::msg(format!(
                "Shuriken directory not found: {}",
                shuriken_dir.display()
            )));
        }

        debug!("Starting process for shuriken: {}", normalized_name);
        if let Err(e) = shuriken
            .start(
                &*self.engine.lock().await,
                &shuriken_dir,
                Some(self.clone()),
            )
            .await
        {
            warn!("Failed to start shuriken '{}': {}", name, e);
            return Err(anyhow::Error::msg(format!(
                "Failed to start shuriken '{}': {}",
                name, e
            )));
        }

        self.update_state(shuriken, ShurikenState::Running).await;
        info!("Successfully started shuriken: {}", name);
        Ok(())
    }

    /// Reloads all Shurikens from disk.
    ///
    /// Rescans the ~/.ninja/shurikens directory and updates the in-memory cache.
    /// Useful after manual file changes or to get latest state from disk.
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err` if file system operations fail
    pub async fn refresh(&self) -> Result<()> {
        info!("Refreshing shurikens from disk");
        let new_shurikens = load_shurikens(&self.root_path).await?;
        let count = new_shurikens.len();
        *self.shurikens.write().await = new_shurikens;
        info!("Shuriken manager refreshed. Found {} shurikens.", count);
        Ok(())
    }

    /// Configures a Shuriken using its configuration script.
    ///
    /// Executes the Shuriken's `post_config` function to apply configuration settings.
    /// Configuration values are templated and written to the Shuriken's config file.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to configure
    ///
    /// # Returns
    /// - `Ok(())` if configuration completed successfully
    /// - `Err` if Shuriken not found or configuration fails
    pub async fn configure_shuriken(&self, name: &str) -> Result<()> {
        info!("Configuring shuriken: {}", name);
        let normalized_name = normalize_shuriken_name(name);
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(&normalized_name);

        if let Some(shuriken) = shuriken {
            let path = &self.root_path;
            shuriken
                .configure(path, &*self.engine.lock().await, Some(self.clone()))
                .await?
        } else {
            warn!("Shuriken not found for configuration: {}", name);
        }
        Ok(())
    }

    /// Removes the lock file for a Shuriken.
    ///
    /// Forces the Shuriken to be considered "not running" by removing its lock file.
    /// Useful for recovering from crashed processes.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken
    ///
    /// # Returns
    /// - `Ok(())` if lock file successfully removed or didn't exist
    /// - `Err` if operation fails
    pub async fn lockpick(&self, name: &str) -> Result<()> {
        info!("Lockpicking shuriken: {}", name);
        let normalized_name = normalize_shuriken_name(name);
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(&normalized_name);

        if let Some(shuriken) = shuriken {
            let path = &self.root_path;
            shuriken.lockpick(path).await?
        } else {
            warn!("Shuriken not found for lockpick: {}", name);
        }
        Ok(())
    }

    pub async fn save_config(&self) -> Result<()> {
        let path = &self.root_path.join("config.toml");
        let data = &self.config.read().await.clone();
        let serialized_data = toml::to_string_pretty(data)?;
        if !path.exists() {
            let mut file = File::create(path).await?;
            file.write_all(serialized_data.as_bytes()).await?;
        } else {
            fs::remove_file(path).await?;
            let mut file = File::create(path).await?;
            file.write_all(serialized_data.as_bytes()).await?;
        }
        Ok(())
    }

    /// Saves configuration options for a Shuriken.
    ///
    /// Persists configuration to disk as TOML and updates the in-memory cache.
    /// Creates necessary directories if they don't exist.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken
    /// - `data`: Configuration key-value pairs to save
    ///
    /// # Returns
    /// - `Ok(())` if configuration saved successfully
    /// - `Err` if file operations fail
    pub async fn save_shuriken_config(
        &self,
        name: &str,
        data: HashMap<String, FieldValue>,
    ) -> Result<()> {
        info!("Saving config for shuriken: {}", name);
        debug!("Config data: {:#?}", data);
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

    /// Stops a running Shuriken.
    ///
    /// Executes the Shuriken's stop script and halts the service.
    /// Updates the Shuriken's state to `Idle` on success.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to stop
    ///
    /// # Returns
    /// - `Ok(())` if stop completed successfully
    /// - `Err` if Shuriken not found or stop script fails
    pub async fn stop(&self, name: &str) -> Result<()> {
        let normalized_name = normalize_shuriken_name(name);
        let shurikens = self.shurikens.read().await;
        let mut shuriken = shurikens
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

        self.update_state(shuriken, ShurikenState::Idle).await;
        Ok(())
    }

    /// Retrieves a Shuriken by name.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to retrieve
    ///
    /// # Returns
    /// - `Ok(Shuriken)` if found
    /// - `Err` if Shuriken not found
    pub async fn get(&self, name: String) -> Result<Shuriken> {
        debug!("Getting shuriken: {}", name);
        let partial_shuriken = &self.shurikens.read().await;
        let maybe_shuriken = partial_shuriken.get(&name);
        info!(
            "Getting shuriken '{}' from shurikens: {}",
            name,
            partial_shuriken
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(", ")
        );
        if let Some(shuriken) = maybe_shuriken {
            debug!("Shuriken metadata: {:?}", shuriken.metadata);
            debug!("Shuriken config: {:?}", shuriken.config);
            Ok(shuriken.clone())
        } else {
            warn!("Shuriken '{}' not found", name);
            Err(anyhow::Error::msg(format!(
                "No shuriken of name {} found",
                name
            )))
        }
    }

    /// Lists all available Shurikens.
    ///
    /// # Arguments
    /// - `state`: If `true`, returns names with their current state; if `false`, returns only names
    ///
    /// # Returns
    /// - `Ok(Left(vec))` with state information if `state` is true
    /// - `Ok(Right(vec))` with just names if `state` is false
    /// - `Err` if operation fails
    pub async fn list(
        &self,
        state: bool,
    ) -> Result<Either<Vec<(String, ShurikenState)>, Vec<String>>> {
        if state {
            let shurikens = self.shurikens.read().await;
            let futures = shurikens
                .iter()
                .map(async |(name, shuriken)| {
                    let state = shuriken.state.lock().await;
                    (name.clone(), state.clone())
                })
                .collect::<Vec<_>>();

            let values = join_all(futures).await;

            debug!("Listing shurikens with state: {:?}", values);
            Ok(Left(values))
        } else {
            let shurikens = self.shurikens.read().await;
            let keys: Vec<String> = shurikens.keys().cloned().collect();
            debug!("Listing shuriken names: {:?}", keys);
            Ok(Right(keys))
        }
    }

    /// Creates a new DSL engine for flow/repl execution.
    ///
    /// # Returns
    /// A `DslEngine` that can be used to interpret Ninja DSL commands
    pub fn new_dsl(&self) -> DslEngine {
        DslEngine {
            selected: Arc::new(RwLock::new(None)),
            manager: self.clone(),
        }
    }

    /// Packages a Shuriken into a distributable `.shuriken` file.
    ///
    /// Creates a signed archive containing metadata, the Shuriken directory, and SHA256 checksum.
    /// Format: MAGIC + metadata_length + metadata + archive_length + archive + signature
    ///
    /// # Arguments
    /// - `meta`: Metadata for the packaged Shuriken
    /// - `path`: Path to the Shuriken directory to package
    /// - `output`: Optional output directory (defaults to ~/.ninja/blacksmith)
    ///
    /// # Returns
    /// - `Ok(())` if packaging succeeded
    /// - `Err` if metadata is too large, archive creation fails, or I/O fails
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

        let mut serialized_metadata = Vec::new();

        into_writer(&meta, &mut serialized_metadata)?;

        if serialized_metadata.len() > u16::MAX as usize {
            return Err(anyhow::Error::msg(
                "Metadata too large to fit in u16 length field",
            ));
        }

        // ---- 2) Build archive bytes (tar.gz) in a non-blocking thread for some reason ----
        let path_clone = path.to_path_buf();

        let archive =
            tokio::task::spawn(async move { create_tar_gz_bytes(path_clone).await }).await??;
        let archive_len: u64 = archive.len().try_into()?;
        // Define a reasonable upper bound to protect system memory (e.g., 5E GB)
        const MAX_ARCHIVE_SIZE: u64 = 5 * 1024 * 1024 * 1024;

        if archive_len > MAX_ARCHIVE_SIZE {
            return Err(anyhow::Error::msg(
                "Archive exceeds the maximum allowable size limit.",
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
        // [archive_length]        // u64 LE
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

    /// Removes a Shuriken from the system.
    ///
    /// Deletes the Shuriken directory and removes it from the cache.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to remove
    ///
    /// # Returns
    /// - `Ok(())` if removal succeeded
    /// - `Err` if Shuriken not found or deletion fails
    pub async fn remove(&self, name: &str) -> Result<()> {
        info!("Removing shuriken: {}", name);
        let normalized_name = normalize_shuriken_name(name);
        warn!("Deleting {}.", name);
        fs::remove_dir_all(format!("shurikens/{}", normalized_name)).await?;
        let _ = &self.shurikens.write().await.remove(&normalized_name);
        info!("Successfully deleted shuriken {}, refreshing.", name);
        #[cfg(debug_assertions)]
        dbg!("{:#?}", &self.shurikens);
        Ok(())
    }

    /// Resets and reinitializes the Lua scripting engine.
    ///
    /// Useful when you need to clear engine state between operations.
    /// Creates a new engine instance with all modules.
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err` if engine initialization fails
    pub async fn reset_engine(&self) -> Result<()> {
        let new_engine = NinjaEngine::new()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        *self.engine.lock().await = new_engine; // don't ask i need to reset the engine everytime i run scripts in gui.
        Ok(())
    }

    // -------------------- Installation functions --------------------

    /// Installs a Shuriken from various sources.
    ///
    /// Automatically detects the source type and installs accordingly:
    /// - Registry reference (e.g., "registry:shuriken")
    /// - Direct URL
    /// - Local file path
    ///
    /// # Arguments
    /// - `source`: The Shuriken source (reference, URL, or file path)
    ///
    /// # Returns
    /// - `Ok(())` if installation completed
    /// - `Err` if source is invalid or installation fails
    pub async fn install<R>(&self, source: &str, report: R) -> Result<()>
    where
        R: Reporter + Send + Sync + 'static,
    {
        if ShurikenReference::parse(&source).is_ok() {
            let reference = ShurikenReference::parse(&source)?;
            self.install_from_registry(&reference, report).await
        } else if source.starts_with("http://") || source.starts_with("https://") {
            self.install_url(&source, report).await
        } else {
            let arc_tx = Arc::new(report);
            self.install_file(&PathBuf::from(source), arc_tx).await
        }
    }

    /// Installs a Shuriken from a direct URL.
    ///
    /// Downloads the .shuriken file and installs it.
    ///
    /// # Arguments
    /// - `url`: The download URL for the .shuriken file
    ///
    /// # Returns
    /// - `Ok(())` if installation succeeded
    /// - `Err` if download or installation fails
    pub async fn install_url<R>(&self, url: &str, tx: R) -> Result<()>
    where
        R: Reporter + Send + Sync + 'static,
    {
        let temp_path = self.root_path.join("temp_shuriken.shuriken");
        download_shuriken(&temp_path, url, &tx).await?;
        let arc_tx = Arc::new(tx);
        let result = self.install_file(&temp_path, arc_tx).await;
        let _ = fs::remove_file(temp_path).await; // clean up temp file
        result
    }

    /// Install a shuriken from a registry reference (e.g., "my-registry:my-shuriken")
    pub async fn install_from_registry<R>(
        &self,
        reference: &crate::common::config::ShurikenReference,
        tx: R,
    ) -> Result<()>
    where
        R: Reporter + Send + Sync + 'static,
    {
        let registries = self.config.read().await.registries.clone();
        let download_url =
            crate::common::config::resolve_download_url(&registries, reference).await?;
        info!(
            "Installing shuriken {} from {}",
            reference.shuriken, download_url
        );
        self.install_url(&download_url, tx).await
    }

    /// Installs a Shuriken from a local file.
    ///
    /// Validates the .shuriken file format (magic bytes, metadata, checksum),
    /// extracts the archive, verifies platform compatibility, and runs postinstall hooks.
    ///
    /// # Arguments
    /// - `path`: Path to the .shuriken file
    ///
    /// # Returns
    /// - `Ok(())` if installation succeeded
    /// - `Err` if file is invalid, corrupted, incompatible, or extraction fails
    ///
    /// # File Format
    /// - MAGIC (6 bytes): "HSRZEG"
    /// - metadata_length (u16 LE)
    /// - metadata (CBOR encoded)
    /// - archive_length (u32 LE)  
    /// - archive (tar.gz)
    /// - signature (32 bytes SHA256)
    pub async fn install_file<R>(&self, path: &Path, tx: Arc<R>) -> Result<(), anyhow::Error>
    where
        R: Reporter + Send + Sync + 'static,
    {
        use sha2::{Digest, Sha256};
        use std::io::Cursor;

        info!("Starting installation of {:?}", path);
        if !path.exists() {
            return Err(anyhow::Error::msg("Path does not exist"));
        }

        let mut file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| io::Error::other(format!("Failed to open shuriken file: {e}")))?;

        tx.stage(InstallStage::Validating)?;
        tx.progress(0)?;

        // 1) MAGIC (6 bytes)
        let mut magic_buf = [0u8; 6];
        file.read_exact(&mut magic_buf).await?;
        if &magic_buf != MAGIC {
            return Err(anyhow::Error::msg("Invalid shuriken file (bad MAGIC)."));
        }

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
            from_reader(metadata_buf.as_slice()).context("Failed to parse metadata CBOR")?;

        info!("Metadata parsing complete");

        debug!("MAGIC:     {:?}", magic_buf);
        debug!("Metadata Length:  {}", metadata_length);
        debug!("Metadata:  {:#?}", metadata);

        // 4) archive_length (u64 LE)
        let mut archive_len_buf = [0u8; 8];
        file.read_exact(&mut archive_len_buf).await?;
        let archive_length = u64::from_le_bytes(archive_len_buf) as usize;

        debug!("Archive Length Buffer = {:?}", archive_len_buf);
        debug!("Archive Length = {}", archive_length);

        const MAX_ARCHIVE: usize = 10 * 1024 * 1024 * 1024; // 10 GB
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

        tx.stage(InstallStage::Extracting)?;
        tx.progress(20)?;

        // Unpack archive in blocking task
        let archive_cursor = Cursor::new(archive_buf);
        let archive_name = normalize_shuriken_name(&metadata.name);
        let unpack_path = self.root_path.clone().join("shurikens").join(&archive_name);
        let root_path = self.root_path.clone().join("shurikens").join(&archive_name);
        let thread_tx = tx.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let gz_decoder = GzDecoder::new(archive_cursor);
            let mut archive = tar::Archive::new(gz_decoder);
            let entries = archive.entries()?;
            let total = entries.size_hint().0.max(1);

            let mut count = 0;

            for entry in entries {
                let mut entry = entry?;

                entry.unpack_in(&unpack_path)?;

                count += 1;

                let progress =
                    (20.0 + (count as f64 / total as f64 * 60.0)).clamp(20.0, 80.0) as u8;

                thread_tx.progress(progress)?;
            }
            Ok(())
        })
        .await??;

        tx.stage(InstallStage::PostInstall)?;
        tx.progress(90)?;

        // Run postinstall script if present
        if let Some(pi_script) = &metadata.postinstall {
            info!("Running postinstall script");
            let path = root_path.join(pi_script);
            let engine = &self.engine.lock().await;
            engine
                .execute_file(&path, Some(&root_path), Some(self.clone()))
                .await?;
        }

        // save config so the paths are correct when we launch.
        self.refresh().await?;
        debug!("Shurikens currently: {:#?}", self.list(false).await);
        if let Some(shuriken) = self.shurikens.read().await.get(&metadata.id)
            && shuriken.config.is_some()
        {
            self.configure_shuriken(&metadata.name).await?;
        }

        tx.stage(InstallStage::Installed)?;
        tx.progress(100)?;

        Ok(())
    }

    /// Fetches all available registries.
    ///
    /// # Returns
    /// A HashMap matching the registry name to its full `Registry` data.
    pub async fn registry_get_all_registries(&self) -> std::collections::HashMap<String, Registry> {
        let registries = self.config.read().await.registries.clone();
        RegistrySources::new(registries).fetch_all().await
    }

    /// Fetches the specific Registry containing a given Shuriken.
    ///
    /// # Arguments
    /// - `name`: The name of the Shuriken to search for
    ///
    /// # Returns
    /// - `Some(Registry)` containing the found Shuriken
    /// - `None` if the Shuriken isn't found in any registry
    pub async fn registry_get_registry_by_shuriken(&self, name: String) -> Option<Registry> {
        let registries = self.config.read().await.registries.clone();
        RegistrySources::new(registries)
            .find_registry_by_shuriken(&name)
            .await
    }

    // -------------------- Project management API --------------------

    /// Lists all projects in the projects directory.
    ///
    /// # Returns
    /// - `Ok(Vec<String>)` with project names
    /// - `Err` if directory access fails
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
