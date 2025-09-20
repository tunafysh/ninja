use std::{
    collections::HashMap, env, io, path::PathBuf, sync::Arc
};
use tokio::{sync::RwLock, fs};
use globwalk::glob;
use crate::{shuriken::Shuriken, types::ShurikenState};
use either::Either::{self, Right, Left};

#[derive(Clone)]
pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub shurikens: Arc<RwLock<HashMap<String, Shuriken>>>,
    pub states: Arc<RwLock<HashMap<String, ShurikenState>>>,
}

impl ShurikenManager {
    pub async fn new() -> Result<Self, io::Error> {
        let exe_dir = env::current_exe()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .parent()
            .unwrap()
            .to_path_buf();

        let shurikens_dir = exe_dir.join("shurikens");

        // Create shurikens directory if it doesn't exist
        if !shurikens_dir.exists() {
            fs::create_dir(&shurikens_dir).await.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to create directory: {}", e))
            })?;
        }

        let mut shurikens = HashMap::new();
        let mut states = HashMap::new();

        // Convert path to glob pattern string
        let partial_manifest_glob_pattern = shurikens_dir.join("**/.ninja/manifest.toml");

        
        let manifest_glob_pattern = partial_manifest_glob_pattern
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8"))?;

        // Read all manifest.toml files
        for entry in glob(manifest_glob_pattern).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Glob failed: {}", e))
        })? {
            match entry {
                Ok(path) => {
                    println!("{}",path.file_name().display());
                    let partial_path = path.into_path();
                    let manifest_path = partial_path.clone();
                    let manifest_content = fs::read_to_string(manifest_path.clone()).await.map_err(|e| {
                            io::Error::new(io::ErrorKind::Other, format!("Failed to read manifest {}: {}", manifest_path.display(), e))
                    })?;
                    if let Some(path) = partial_path.parent() {       
                        if let Some(parent) = path.parent() {
                            let name = parent.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid directory name"))?;    
                        
                        let manifest: Shuriken = toml::from_str(&manifest_content)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse TOML: {}", e)))?;
                    
                    shurikens.insert(name, manifest);
                }
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Path is not valid UTF-8"))?;

        for entry in glob(lock_glob_pattern).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Glob failed for lockfiles: {}", e))
        })? {
            match entry {
                Ok(path) => {
                    let partial_path = path.into_path();
                    if let Some(path) = partial_path.parent() {
                        if let Some(parent) = path.parent() {
                            match parent.file_name() {
                                Some(name) => {
                                states.insert(name.display().to_string(), ShurikenState::Running);
                                ()
                            }
                            None => (),
                        }
                    }
                    }
                }
                Err(e) => {
                    eprintln!("Invalid lockfile entry: {}", e);
                }
            }
        }

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

    pub async fn start(&self, name: &str) -> Result<(), String> {
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens.get(name).ok_or_else(|| format!("No such shuriken: {}", name))?.clone();
        drop(shurikens); // Drop the lock before await

        let shuriken_dir = self.root_path.join("shurikens").join(name);
        if !shuriken_dir.exists() {
            return Err(format!("Shuriken directory not found: {}", shuriken_dir.display()));
        }

        // Change directory temporarily (but avoid blocking other threads)
        let original_dir = env::current_dir().map_err(|e| e.to_string())?;
        env::set_current_dir(&shuriken_dir).map_err(|e| e.to_string())?;

        // Run async start
        if let Err(e) = shuriken.start().await {
            env::set_current_dir(original_dir).ok();
            return Err(format!("Failed to start shuriken '{}': {}", name, e));
        }

        env::set_current_dir(original_dir).map_err(|e| e.to_string())?;

        self.update_state(name, ShurikenState::Running).await;
        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<(), String> {
        let shurikens = self.shurikens.read().await;
        let shuriken = shurikens.get(name).ok_or_else(|| format!("No such shuriken: {}", name))?.clone();
        drop(shurikens); // Drop the lock

        let shuriken_dir = self.root_path.join("shurikens").join(name);
        if !shuriken_dir.exists() {
            return Err(format!("Shuriken directory not found: {}", shuriken_dir.display()));
        }

        let original_dir = env::current_dir().map_err(|e| e.to_string())?;
        env::set_current_dir(&shuriken_dir).map_err(|e| e.to_string())?;

        if let Err(e) = shuriken.stop().await {
            env::set_current_dir(original_dir).ok();
            return Err(format!("Failed to stop shuriken '{}': {}", name, e));
        }

        env::set_current_dir(original_dir).map_err(|e| e.to_string())?;

        self.update_state(name, ShurikenState::Idle).await;
        Ok(())
    }

    

    pub async fn get(&self, name: String) -> Result<Shuriken, io::Error> {
        let partial_shuriken = &self.shurikens.read().await;
        let maybe_shuriken = partial_shuriken.get(&name);
        if let Some(shuriken) = maybe_shuriken {
            Ok(shuriken.clone())
        }
        else {
            Err(io::Error::new(io::ErrorKind::Other, format!("No shuriken of name {} found", name)))
        }
    }

    pub async fn list(&self, state: bool) -> Result<Either<Vec<(String, ShurikenState)>, Vec<String>>, io::Error> {
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

     pub async fn install(&self, path: PathBuf) -> Result<(), io::Error> {
        if !path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Path does not exist"));
        }

        if !path.ends_with(".shuriken") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid shuriken file type, expected .shuriken",
            ));
        }

        // Open the ZIP archive
        let std_file = std::fs::File::open(&path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open shuriken file: {}", e),
            )
        })?;

        Ok(())
    }

}