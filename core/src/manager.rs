use crate::{shuriken::Shuriken, types::{FieldValue, ShurikenState}};
use anyhow::Result;
use either::Either::{self, Left, Right};
use globwalk::glob;
use log::{debug, info, warn};
use std::{collections::HashMap, env, io, path::PathBuf, sync::Arc};
use tokio::{fs, sync::RwLock};

#[derive(Clone, Debug)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub states: Arc<RwLock<HashMap<String, ShurikenState>>>,
}

impl ShurikenManager {
    pub async fn new() -> Result<Self> {
        let exe_dir = env::current_exe()
            .map_err(io::Error::other)?
            .parent()
            .unwrap()
            .to_path_buf();

        let shurikens_dir = exe_dir.join("shurikens");

        // Create shurikens directory if it doesn't exist
        if !shurikens_dir.exists() {
            fs::create_dir(&shurikens_dir)
                .await
                .map_err(|e| io::Error::other(format!("Failed to create directory: {}", e)))?;
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

        for entry in glob(options_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let options_path = partial_path.clone();
                    let options_content = fs::read_to_string(options_path.clone())
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

                        let options: HashMap<String, FieldValue> =
                            toml::from_str(&options_content).map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        let shuriken = shurikens.get_mut(&name);
                        if let Some(shuriken) = shuriken
                        && let Some(config) = &mut shuriken.config {
                            config.options = Some(options);
                        }

                    }
                }
                Err(e) => {
                    eprintln!("Invalid glob entry: {}", e);
                }
            }
        }

        debug!("{:#?}", shurikens);

        Ok(Self {
            root_path: exe_dir,
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
            return Err(anyhow::Error::msg(format!("Failed to start shuriken '{}': {}", name, e)));
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

        for entry in glob(options_pattern)
            .map_err(|e| io::Error::other(format!("Glob failed: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    let options_path = partial_path.clone();
                    let options_content = fs::read_to_string(options_path.clone())
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

                        let options: HashMap<String, FieldValue> =
                            toml::from_str(&options_content).map_err(|e| {
                                println!("{}", e);
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?;

                        let shuriken = new_shurikens.get_mut(&name);
                        if let Some(shuriken) = shuriken
                        && let Some(config) = &mut shuriken.config {
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
        let temp_path = &self.root_path.clone().join(PathBuf::from(format!("shurikens/{}", name)));
        env::set_current_dir(temp_path)?;
        let partial_shuriken = &self.shurikens.write().await;
        let shuriken = partial_shuriken.get(name);

        if let Some(shuriken) = shuriken {
            shuriken.configure().await?;
        }
        env::set_current_dir(&self.root_path)?;
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
            return Err(anyhow::Error::msg(format!("Failed to stop shuriken '{}': {}", name, e)));
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

            println!("{:#?}", values);
            Ok(Left(values))
        } else {
            let mut values: Vec<String> = Vec::new();
            let map = &self.shurikens.read().await.clone();

            for key in map.keys() {
                values.push(key.clone())
            }

            println!("{:#?}", values);

            Ok(Right(values))
        }
    }

    pub async fn install(&self, path: PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::Error::msg("Path does not exist"));
        }

        if !path.ends_with(".shuriken") {
            return Err(anyhow::Error::msg(
                "Invalid shuriken file type, expected .shuriken",
            ));
        }

        let _std_file = std::fs::File::open(&path)
            .map_err(|e| io::Error::other(format!("Failed to open shuriken file: {}", e)))?;

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
}
