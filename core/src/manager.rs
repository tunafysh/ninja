use crate::{
    dsl::DslContext,
    scripting::NinjaEngine,
    shuriken::{Shuriken, ShurikenConfig},
    types::{FieldValue, ShurikenState},
};
use anyhow::{Error, Result};
use dirs_next as dirs;
use either::Either::{self, Left, Right};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use globwalk::{GlobWalkerBuilder, glob};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_cbor::{de::from_mut_slice, to_vec};
use std::{
    collections::HashMap,
    env,
    io::{self, Cursor, Read, Seek},
    path::PathBuf,
    str,
    sync::Arc,
};
use tar::{Archive, Builder as TarBuilder};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::RwLock,
};

const MAGIC: &[u8; 2] = b"HS";

fn create_tar_gz_bytes(src_dir: &PathBuf) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let enc = GzEncoder::new(&mut buf, Compression::default());
    let mut tar = TarBuilder::new(enc);

    // Iterate files using globwalk
    let walker = GlobWalkerBuilder::from_patterns(src_dir, &["**/*"])
        .build()
        .map_err(|e| Error::msg(e.to_string()))?;

    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        let rel_path = path.strip_prefix(src_dir).unwrap();

        if path.is_dir() {
            tar.append_dir(rel_path, path)?;
        } else if path.is_file() {
            tar.append_path_with_name(path, rel_path)?;
        }
    }

    // Finish tar and gzip streams
    tar.finish()?;
    tar.into_inner()?.finish()?;

    Ok(buf)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArmoryMetadata {
    pub name: String,
    pub id: String,
    pub platform: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub postinstall: Option<PathBuf>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub engine: NinjaEngine,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub states: Arc<RwLock<HashMap<String, ShurikenState>>>,
}

impl ShurikenManager {
    pub async fn new() -> Result<Self> {
        let exe_dir = if cfg!(target_os = "linux") {
            // Linux: ~/.ninja
            dirs::home_dir()
                .ok_or_else(|| Error::msg("Could not find home directory"))?
                .join(".ninja")
        } else {
            // Windows/macOS: use local app data dir
            dirs::data_local_dir()
                .ok_or_else(|| Error::msg("No local data dir found"))?
                .join("com.tunafysh.ninja")
        };

        fs::create_dir_all(&exe_dir).await?;

        env::set_current_dir(&exe_dir)?;

        let shurikens_dir = exe_dir.join("shurikens");
        let projects_dir = exe_dir.join("projects");

        // Create shurikens directory if it doesn't exist
        if !shurikens_dir.exists() {
            fs::create_dir(&shurikens_dir).await?;
        }

        if !projects_dir.exists() {
            fs::create_dir(&projects_dir).await?;
        }

        let mut shurikens = HashMap::new();
        let mut states = HashMap::new();

        // Convert path to glob pattern string
        let partial_manifest_glob_pattern = shurikens_dir.join("**/.ninja/manifest.toml");

        let manifest_glob_pattern = partial_manifest_glob_pattern.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8")
        })?;

        // ===== Read all manifest.toml files =====
        for entry in glob(manifest_glob_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let manifest_path = partial_path.clone();
                    let manifest_content = fs::read_to_string(manifest_path.clone())
                        .await
                        .map_err(|e| {
                            io::Error::other(format!(
                                "Failed to read manifest {}: {}",
                                manifest_path.display(),
                                e
                            ))
                        })?;

                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                    {
                        let name = parent
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "Invalid directory name",
                                )
                            })?;

                        let manifest: Shuriken =
                            toml::from_str(&manifest_content).map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        shurikens.insert(name.clone(), manifest);
                        states.insert(name, ShurikenState::Idle);
                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob entry: {}", e);
                }
            }
        }

        // Load lock files to determine running states
        let partial_lock_glob_pattern = shurikens_dir.join("**/.ninja/shuriken.lck");

        let lock_glob_pattern = partial_lock_glob_pattern
            .to_str()
            .ok_or_else(|| io::Error::other("Path is not valid UTF-8"))?;

        for entry in glob(lock_glob_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed for lockfiles: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                        && let Some(name) = parent.file_name()
                    {
                        states.insert(name.display().to_string(), ShurikenState::Running);
                    }
                }
                Err(e) => {
                    eprintln!("Invalid lockfile entry: {}", e);
                }
            }
        }

        // Load options if any

        let partial_options_pattern = shurikens_dir.join("**/.ninja/options.toml");
        let options_pattern = partial_options_pattern.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8")
        })?;

        for entry in
            glob(options_pattern).map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let options_path = partial_path.clone();
                    let options_content =
                        fs::read_to_string(options_path.clone())
                            .await
                            .map_err(|e| {
                                io::Error::other(format!(
                                    "Failed to read options {}: {}",
                                    options_path.display(),
                                    e
                                ))
                            })?;

                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                    {
                        let name = parent
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "Invalid directory name",
                                )
                            })?;

                        let options: HashMap<String, FieldValue> = toml::from_str(&options_content)
                            .map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        let shuriken = shurikens.get_mut(&name);
                        if let Some(shuriken) = shuriken
                            && let Some(config) = &mut shuriken.config
                        {
                            config.options = Some(options);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob entry: {}", e);
                }
            }
        }

        let engine = NinjaEngine::new();

        Ok(Self {
            root_path: exe_dir,
            engine: engine.map_err(|e| Error::msg(e.to_string()))?,
            shurikens: Arc::new(RwLock::new(shurikens)),
            states: Arc::new(RwLock::new(states)),
        })
    }

    async fn update_state(&self, name: &str, new_state: ShurikenState) {
        let mut state_map = self.states.write().await;
        state_map.insert(name.to_string(), new_state);
    }

    pub async fn start(&self, name: &str) -> Result<()> {
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens
            .get(name)
            .ok_or_else(|| anyhow::Error::msg(format!("No such shuriken: {}", name)))?
            .clone();
        drop(shurikens); // Drop the lock before await

        let shuriken_dir = self.root_path.join("shurikens").join(name).join(".ninja");
        if !shuriken_dir.exists() {
            return Err(anyhow::Error::msg(format!(
                "Shuriken directory not found: {}",
                shuriken_dir.display()
            )));
        }

        // Change directory temporarily (but avoid blocking other threads)
        let original_dir = env::current_dir().map_err(|e| anyhow::Error::msg(e.to_string()))?;
        env::set_current_dir(&shuriken_dir).map_err(|e| anyhow::Error::msg(e.to_string()))?;

        // Run async start
        if let Err(e) = shuriken.start().await {
            env::set_current_dir(original_dir).ok();
            return Err(anyhow::Error::msg(format!(
                "Failed to start shuriken '{}': {}",
                name, e
            )));
        }

        env::set_current_dir(original_dir).map_err(|e| anyhow::Error::msg(e.to_string()))?;

        self.update_state(name, ShurikenState::Running).await;
        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        let exe_dir = self.root_path.clone();
        let shurikens_dir = exe_dir.join("shurikens");

        // Create shurikens directory if it doesn't exist
        if !shurikens_dir.exists() {
            fs::create_dir(&shurikens_dir)
                .await
                .map_err(|e| io::Error::other(format!("Failed to create directory: {}", e)))?;
        }

        let mut new_shurikens = HashMap::new();
        let mut new_states = HashMap::new();

        // ===== Read all manifest.toml files =====
        let partial_manifest_glob_pattern = shurikens_dir.join("**/.ninja/manifest.toml");
        let manifest_glob_pattern = partial_manifest_glob_pattern.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8")
        })?;

        for entry in glob(manifest_glob_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let manifest_path = partial_path.clone();
                    let manifest_content = fs::read_to_string(manifest_path.clone())
                        .await
                        .map_err(|e| {
                            io::Error::other(format!(
                                "Failed to read manifest {}: {}",
                                manifest_path.display(),
                                e
                            ))
                        })?;

                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                    {
                        let name = parent
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "Invalid directory name",
                                )
                            })?;

                        let manifest: Shuriken =
                            toml::from_str(&manifest_content).map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        new_shurikens.insert(name.clone(), manifest);
                        new_states.insert(name, ShurikenState::Idle);
                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob entry: {}", e);
                }
            }
        }

        // ===== Load lock files =====
        let partial_lock_glob_pattern = shurikens_dir.join("**/.ninja/shuriken.lck");
        let lock_glob_pattern = partial_lock_glob_pattern
            .to_str()
            .ok_or_else(|| io::Error::other("Path is not valid UTF-8"))?;

        for entry in glob(lock_glob_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed for lockfiles: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                        && let Some(name) = parent.file_name()
                    {
                        new_states.insert(name.display().to_string(), ShurikenState::Running);
                    }
                }
                Err(e) => {
                    eprintln!("Invalid lockfile entry: {}", e);
                }
            }
        }

        // ===== Load options =====
        let partial_options_pattern = shurikens_dir.join("**/.ninja/options.toml");
        let options_pattern = partial_options_pattern.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8")
        })?;

        for entry in
            glob(options_pattern).map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let options_path = partial_path.clone();
                    let options_content =
                        fs::read_to_string(options_path.clone())
                            .await
                            .map_err(|e| {
                                io::Error::other(format!(
                                    "Failed to read options {}: {}",
                                    options_path.display(),
                                    e
                                ))
                            })?;

                    if let Some(path) = partial_path.parent()
                        && let Some(parent) = path.parent()
                    {
                        let name = parent
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "Invalid directory name",
                                )
                            })?;

                        let options: HashMap<String, FieldValue> = toml::from_str(&options_content)
                            .map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        let shuriken = new_shurikens.get_mut(&name);
                        if let Some(shuriken) = shuriken
                            && let Some(config) = &mut shuriken.config
                        {
                            config.options = Some(options);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob entry: {}", e);
                }
            }
        }

        // Replace old maps atomically
        {
            let mut shurikens_lock = self.shurikens.write().await;
            *shurikens_lock = new_shurikens;
        }
        {
            let mut states_lock = self.states.write().await;
            *states_lock = new_states;
        }

        debug!("Shuriken manager refreshed successfully.");
        Ok(())
    }

    pub async fn configure(&self, name: &str) -> Result<()> {
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(name);

        if let Some(shuriken) = shuriken {
            let path = &self.root_path;
            shuriken.configure(path.clone()).await?;
        }
        Ok(())
    }

    pub async fn save_config(&self, name: &str, data: HashMap<String, FieldValue>) -> Result<()> {
        println!("{:#?}", data);

        // Update in-memory config
        {
            let mut shurikens = self.shurikens.write().await;
            if let Some(shuriken) = shurikens.get_mut(name) {
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
            .join(name)
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
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens
            .get(name)
            .ok_or_else(|| anyhow::Error::msg(format!("No such shuriken: {}", name)))?
            .clone();
        drop(shurikens); // Drop the lock

        let shuriken_dir = self.root_path.join("shurikens").join(name).join(".ninja");
        if !shuriken_dir.exists() {
            return Err(anyhow::Error::msg(format!(
                "Shuriken directory not found: {}",
                shuriken_dir.display()
            )));
        }

        let original_dir = env::current_dir().map_err(|e| anyhow::Error::msg(e.to_string()))?;
        env::set_current_dir(&shuriken_dir).map_err(|e| anyhow::Error::msg(e.to_string()))?;

        if let Err(e) = shuriken.stop().await {
            env::set_current_dir(original_dir).ok();
            return Err(anyhow::Error::msg(format!(
                "Failed to stop shuriken '{}': {}",
                name, e
            )));
        }

        env::set_current_dir(original_dir).map_err(|e| anyhow::Error::msg(e.to_string()))?;

        self.update_state(name, ShurikenState::Idle).await;
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
            let mut values: Vec<(String, ShurikenState)> = Vec::new();
            let partial_states = &self.states.read().await.clone().to_owned();

            for (k, v) in partial_states.iter() {
                values.push((k.clone(), v.clone()));
            }
            Ok(Left(values))
        } else {
            let mut values: Vec<String> = Vec::new();
            let map = &self.shurikens.read().await.clone();

            for key in map.keys() {
                values.push(key.clone())
            }

            Ok(Right(values))
        }
    }

    pub fn dsl_ctx(&self) -> DslContext {
        DslContext {
            selected: Arc::new(RwLock::new(None)),
            manager: self.clone(),
        }
    }

    pub async fn install(&self, path: PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::Error::msg("Path does not exist"));
        }

        let mut file = std::fs::File::open(&path)
            .map_err(|e| io::Error::other(format!("Failed to open shuriken file: {}", e)))?;

        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;

        if &header[0..2] != MAGIC {
            return Err(anyhow::Error::msg("Invalid shuriken file."));
        }

        let metadata_length = u16::from_le_bytes([header[3], header[4]]);

        let mut metadata = vec![0u8; metadata_length.into()];
        file.read_exact(&mut metadata)?;

        let metadata: ArmoryMetadata = from_mut_slice(&mut metadata)?;

        info!("Metadata parsing complete");

        if !metadata.platform.contains(env::consts::OS)
            && !metadata.platform.contains(env::consts::ARCH)
        {
            return Err(Error::msg(
                "Unsupported platform. Current platform is not the same with the shuriken's destined platform.",
            ));
        }

        file.seek(io::SeekFrom::Current(metadata_length as i64))?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let buf = Cursor::new(buf);
        let gz_decoder = GzDecoder::new(buf);
        let mut archive = Archive::new(gz_decoder);

        archive.unpack(metadata.name.clone())?;

        if let Some(pi_script) = &metadata.postinstall {
            info!("Running postinstall script");

            let _ = &self.engine.execute_file(pi_script)?;
        }

        #[cfg(debug_assertions)]
        {
            dbg!("{:#?}", header);
            dbg!("{:#?}", metadata_length);
            dbg!("{:#?}", metadata);
        }

        Ok(())
    }

    pub async fn forge(&self, meta: ArmoryMetadata, path: PathBuf, output: PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::Error::msg("Source folder does not exist"));
        }

        let mut file = File::create(output.join(format!("{}-{}", meta.id, meta.platform))).await?;
        let serialized_metadata = to_vec(&meta)?;

        let archive = create_tar_gz_bytes(&path)?;

        file.write_all(MAGIC).await?;
        file.write_all(&(serialized_metadata.len() as u16).to_le_bytes())
            .await?;
        file.write_all(&serialized_metadata).await?;
        file.write_all(&archive).await?;

        Ok(())
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        warn!("Deleting {}.", name);
        fs::remove_dir_all(format!("shurikens/{}", name)).await?;
        let _ = &self.shurikens.write().await.remove(name);
        info!("Successfully deleted shuriken {}, refreshing.", name);
        #[cfg(debug_assertions)]
        dbg!("{:#?}", &self.shurikens);
        Ok(())
    }

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
