#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod fs_api {
    use std::env::consts;
    use std::fs;
    use std::path::Path;
    use rquickjs::Result;

    pub const platform: &str = consts::OS;

    // Individual fs functions - now return Result types
    #[rquickjs::function]
    pub fn read(path: String) -> Result<String> {
        fs::read_to_string(path)
            .map_err(|_ae| rquickjs::Error::new_from_js("fs", "Failed to read file."))
    }

    #[rquickjs::function]
    pub fn write(path: String, content: String) -> Result<()> {
        fs::write(path, content)
            .map_err(|_e| rquickjs::Error::new_from_js("fs", "Failed to write file."))
    }

    #[rquickjs::function]
    pub fn exists(path: String) -> bool {
        Path::new(&path).exists()
    }

    #[rquickjs::function]
    pub fn mkdir(path: String) -> Result<()> {
        fs::create_dir_all(path)
            .map_err(|_e| rquickjs::Error::new_from_js("fs", "Failed to create directory."))
    }

    #[rquickjs::function]
    pub fn remove(path: String) -> Result<()> {
        let path = Path::new(&path);
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        result.map_err(|_e| rquickjs::Error::new_from_js("fs", "Failed to remove: {}"))
    }

    // Module initialization - corrected attribute syntax
    
}