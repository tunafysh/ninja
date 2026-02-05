use crate::{
    dsl::DslContext,
    scripting::NinjaEngine,
    shuriken::{Shuriken, ShurikenConfig},
    types::{FieldValue, ShurikenState},
};
use anyhow::{Context, Error, Result};
use dirs_next as dirs;
use either::Either::{self, Left, Right};
use flate2::{Compression, write::GzEncoder};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_cbor::to_vec;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
    str,
    sync::Arc,
};
use tar::Builder as TarBuilder;
use tokio::process::{Child, Command};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    sync::RwLock,
};

const MAGIC: &[u8; 6] = b"HSRZEG";

/// Normalizes a shuriken name to lowercase for consistent directory naming.
/// This ensures all shuriken directories use lowercase names.
fn normalize_shuriken_name(name: &str) -> String {
    name.to_lowercase()
}

fn create_tar_gz_bytes(src_dir: &Path) -> Result<Vec<u8>> {
    if !src_dir.is_dir() {
        return Err(anyhow::Error::msg(format!(
            "Source directory does not exist or is not a directory: {}",
            src_dir.display()
        )));
    }

    let mut buf = Vec::new();

    {
        // Gzip wraps the in-memory buffer
        let enc = GzEncoder::new(&mut buf, Compression::default());
        let mut tar = TarBuilder::new(enc);

        // This recursively adds `src_dir` contents under "." in the archive
        tar.append_dir_all(".", src_dir)?;

        // Finish tar, then finish gzip
        let enc = tar.into_inner()?; // GzEncoder
        enc.finish()?; // flush into buf
    }

    Ok(buf)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArmoryMetadata {
    pub name: String,
    pub id: String,
    pub platform: String,
    pub version: String,
    pub synopsis: Option<String>,
    pub postinstall: Option<PathBuf>,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
}

/// A thin wrapper around a spawned process. We keep it simple: the
/// ManagedProcess owns a `tokio::process::Child` and provides async helpers.
#[derive(Debug)]
pub struct ManagedProcess {
    pub child: Child,
    pub cmd: String,
    pub args: Vec<String>,
}

impl ManagedProcess {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Attempt to check if the process is still running. Returns `Ok(true)` when running.
    pub fn is_running_sync(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    /// Kill and await the child process.
    pub async fn kill_and_wait(&mut self) -> Result<()> {
        // kill() is synchronous (returns io::Result), wait is async
        self.child
            .kill()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        let _ = self
            .child
            .wait()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub engine: Arc<Mutex<NinjaEngine>>,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub states: Arc<RwLock<HashMap<String, ShurikenState>>>,
    /// Manage runtime processes for services started by Ninja
    pub processes: Arc<RwLock<HashMap<String, Arc<Mutex<ManagedProcess>>>>>,
}

impl ShurikenManager {
    // Shared logic for loading shurikens from disk
    async fn load_shurikens(
        root_path: &Path,
    ) -> Result<(HashMap<String, Shuriken>, HashMap<String, ShurikenState>)> {
        let shurikens_dir = root_path.join("shurikens");
        let mut shurikens = HashMap::new();
        let mut states = HashMap::new();

        // Only iterate immediate children of `shurikens/`
        let mut dir = match fs::read_dir(&shurikens_dir).await {
            Ok(d) => d,
            Err(_) => return Ok((shurikens, states)), // no shurikens dir = empty
        };

        while let Some(entry) = dir.next_entry().await? {
            let shuriken_path = entry.path();
            if !shuriken_path.is_dir() {
                continue;
            }

            let name = match shuriken_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_owned(),
                None => continue, // skip non-UTF8 names
            };

            let ninja_dir = shuriken_path.join(".ninja");

            // 1. Load manifest (required)
            let manifest_path = ninja_dir.join("manifest.toml");
            if !manifest_path.exists() {
                continue; // not a valid shuriken
            }

            let content: String = fs::read_to_string(&manifest_path).await?;
            let mut shuriken: Shuriken = toml::from_str(&content).map_err(|e| {
                Error::msg(format!("TOML error in {}: {}", manifest_path.display(), e))
            })?;

            // 2. Check for lock file
            let lock_path = ninja_dir.join("shuriken.lck");
            let state = if lock_path.exists() {
                ShurikenState::Running
            } else {
                ShurikenState::Idle
            };

            // 3. Load options (optional)
            let options_path = ninja_dir.join("options.toml");
            if options_path.exists() {
                let content: String = fs::read_to_string(&options_path).await?;
                let options: HashMap<String, FieldValue> =
                    toml::from_str(&content).map_err(|e| {
                        Error::msg(format!(
                            "Options error in {}: {}",
                            options_path.display(),
                            e
                        ))
                    })?;

                if let Some(config) = &mut shuriken.config {
                    config.options = Some(options);
                } else {
                    shuriken.config = Some(ShurikenConfig {
                        config_path: PathBuf::from("options.toml"),
                        options: Some(options),
                    });
                }
            }

            // Store using the directory name (which should already be lowercase)
            // but normalize it to be sure
            let normalized_name = normalize_shuriken_name(&name);
            shurikens.insert(normalized_name.clone(), shuriken);
            states.insert(normalized_name, state);
        }

        Ok((shurikens, states))
    }

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

        let (shurikens, states) = Self::load_shurikens(&exe_dir).await?;

        let engine = NinjaEngine::new()
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

        Ok(Self {
            root_path: exe_dir,
            engine: Arc::new(Mutex::new(engine)),
            shurikens: Arc::new(RwLock::new(shurikens)),
            states: Arc::new(RwLock::new(states)),
            processes: Arc::new(RwLock::new(HashMap::new())),
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
                Some(&*self.engine.lock().await),
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
        let (new_shurikens, new_states) = Self::load_shurikens(&self.root_path).await?;
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
                Some(&*self.engine.lock().await),
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

    pub async fn install(&self, path: &Path) -> Result<(), anyhow::Error> {
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
        hasher.update(magic_buf);
        hasher.update(meta_len_buf);
        hasher.update(&metadata_buf);
        hasher.update(archive_len_buf);
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

    pub async fn forge(&self, meta: ArmoryMetadata, path: PathBuf) -> Result<()> {
        let output = self.root_path.join("blacksmith");
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

    // -------------------- Process management API --------------------

    /// Spawn a process and track it under `proc_name`.
    /// `cmd` is the executable, `args` are arguments. `cwd` and `envs` are optional.
    pub async fn spawn_process(
        &self,
        proc_name: &str,
        cmd: &str,
        args: &[String],
        cwd: Option<PathBuf>,
        envs: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let mut command = Command::new(cmd);
        command.args(args);
        if let Some(c) = cwd {
            command.current_dir(c);
        }
        if let Some(map) = envs {
            for (k, v) in map.into_iter() {
                command.env(k, v);
            }
        }

        let child = command.spawn().map_err(|e| Error::msg(e.to_string()))?;

        let managed = ManagedProcess {
            child,
            cmd: cmd.to_string(),
            args: args.to_vec(),
        };

        self.processes
            .write()
            .await
            .insert(proc_name.to_string(), Arc::new(Mutex::new(managed)));
        Ok(())
    }

    /// Stop (kill + wait) a named process and remove it from tracking.
    pub async fn stop_process(&self, proc_name: &str) -> Result<()> {
        let maybe = { self.processes.write().await.remove(proc_name) };
        if let Some(proc_arc) = maybe {
            let mut guard = proc_arc.lock().await;
            let _ = guard.kill_and_wait().await;
        }
        Ok(())
    }

    /// Check whether a tracked process is still running.
    pub async fn is_process_running(&self, proc_name: &str) -> Result<bool> {
        let procs = self.processes.read().await;
        if let Some(proc_arc) = procs.get(proc_name) {
            let mut guard = proc_arc.lock().await;
            Ok(guard.is_running_sync())
        } else {
            Ok(false)
        }
    }

    /// List tracked processes and their basic info.
    pub async fn list_processes(&self) -> Result<Vec<(String, Option<u32>, bool)>> {
        let procs = self.processes.read().await;
        let mut out = Vec::new();
        for (name, arc_proc) in procs.iter() {
            let mut guard = arc_proc.lock().await;
            out.push((name.clone(), guard.id(), guard.is_running_sync()));
        }
        Ok(out)
    }

    /// Kill all tracked processes. This is the async cleanup you should call on shutdown.
    pub async fn cleanup(&self) {
        let keys: Vec<String> = {
            let procs = self.processes.read().await;
            procs.keys().cloned().collect()
        };

        for k in keys {
            let _ = self.stop_process(&k).await;
        }
    }
}
