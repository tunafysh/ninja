use std::{
    collections::HashMap, env, io::{self, Read}, path::{Path, PathBuf}
};
use tokio::{sync::RwLock, fs};
use globwalk::glob;
use crate::{shuriken::Shuriken, types::ShurikenState};
use either::Either::{self, Right, Left};

pub struct ShurikenManager {
    pub root_path: PathBuf,
    pub shurikens: RwLock<HashMap<String, Shuriken>>,
    pub states: RwLock<HashMap<String, ShurikenState>>,
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
                    let partial_path = path.into_path();
                    if let Some(path) = partial_path.parent() {       
                        if let Some(parent) = path.parent() {
                            let name = parent.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_owned())
                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid directory name"))?;

                        let manifest_content = fs::read_to_string(&path).await.map_err(|e| {
                            io::Error::new(io::ErrorKind::Other, format!("Failed to read manifest {}: {}", path.display(), e))
                        })?;

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
            shurikens: RwLock::new(shurikens),
            states: RwLock::new(states),
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

        self.update_state(name, ShurikenState::Stopped).await;
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

        let mut archive = zip::ZipArchive::new(std_file).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read zip archive: {}", e),
            )
        })?;

        let current_target = format!("{}-{}", env::consts::OS, env::consts::ARCH);

        // Installation directory: shurikens/<package-name>
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid shuriken filename"))?;
        let install_dir = PathBuf::from("shurikens").join(stem);

        let mut found_target = false;
        let mut found_root_files = false;

        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to access file in archive: {}", e),
                )
            })?;

            let zip_file_name = zip_file.name();
            let zip_file_path = Path::new(zip_file_name);

            // === Case 1: Root file (no parent or parent is empty) ===
            if zip_file_path.parent().map_or(true, |p| p.as_os_str().is_empty()) {
                // Must be a file (not a directory like "linux-x86_64/")
                if !zip_file.is_dir() && !zip_file_name.contains('/') && !zip_file_name.starts_with('\\') {
                    found_root_files = true;

                    let output_path = install_dir.join(zip_file_name);

                    // Ensure install dir exists
                    fs::create_dir_all(&install_dir).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to create install directory: {}", e))
                    })?;

                    let mut content = Vec::new();
                    zip_file.read_to_end(&mut content).map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("Failed to read root file: {}", e))
                    })?;

                    let mut out_file = fs::File::create(&output_path).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to create output file: {}", e))
                    })?;

                    tokio::io::copy(&mut &content[..], &mut out_file).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to write root file: {}", e))
                    })?;
                }

                continue;
            }

            // === Case 2: Entry inside the target platform directory (e.g. linux-x86_64/bin/app) ===
            let relative_path = zip_file_path.strip_prefix(&current_target).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}",e.to_string())))?;
            
                if relative_path.as_os_str().is_empty() {
                    found_target = true;
                    // Ensure install dir exists (again, safe to call multiple times)
                    fs::create_dir_all(&install_dir).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to create install directory: {}", e))
                    })?;
                    continue;
                }

                found_target = true;
                let output_path = install_dir.join(relative_path);

                if zip_file.is_dir() {
                    fs::create_dir_all(&output_path).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to create directory: {}", e))
                    })?;
            
                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent).await.map_err(|e| {
                            io::Error::new(io::ErrorKind::Other, format!("Failed to create parent directory: {}", e))
                        })?;
                    }

                    let mut content = Vec::new();
                    zip_file.read_to_end(&mut content).map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("Failed to read file from zip: {}", e))
                    })?;

                    let mut out_file = fs::File::create(&output_path).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to create output file: {}", e))
                    })?;

                    tokio::io::copy(&mut &content[..], &mut out_file).await.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to write file: {}", e))
                    })?;
                }
            }
            
            // Final validation
            if !found_target {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "No directory or contents found for target '{}' in the shuriken package",
                    current_target
                ),
            ));
        }

        if !found_root_files {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No files found in the root of the shuriken package (e.g. manifest.toml, README.md)",
            ));
        }

        Ok(())
    }

}