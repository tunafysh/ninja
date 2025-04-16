use std::path::Path;

use tauri::Manager;
mod shuriken;
mod registry;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/\
#[tauri::command]
async fn create_shuriken(name: String, service: String) -> Result<shuriken::Shuriken, String> {
    Ok(shuriken::Shuriken::new(&name, &service))
}

#[tauri::command]
fn bootstrap() -> Result<registry::ShurikenRegistry, String> {
    match registry::ShurikenRegistry::new("./registry.toml") {
        Ok(reg) => {
            println!("Registry created at: {}", reg.registry_path.display());
            return Ok(reg);
        }
        Err(err) => {
            return Err(format!("Failed to create registry: {}", err));
        }
    }
}

#[tauri::command]
async fn control_shuriken(
    shuriken: shuriken::Shuriken,
    action: String,
) -> Result<shuriken::Shuriken, String> {
    let mut s = shuriken;
    match action.as_str() {
        "throw" => s.throw().await?,
        "recall" => s.recall().await?,
        "spin" => s.spin().await?,
        _ => return Err("Invalid action".into()),
    };
    Ok(s)
}

#[tauri::command]
async fn install_shuriken(reg_path: String, config_path: String) -> Result<(), String> {
    match registry::ShurikenRegistry::new(&reg_path) {
        Ok(mut reg) => {
            reg.install(&Path::new(&config_path)).expect("Failed to install shuriken");
            Ok(())
        }
        Err(err) => {
            return Err(format!("Failed to create registry: {}", err));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .filter(|metadata| metadata.target() != "tao")
            .build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![create_shuriken, control_shuriken, install_shuriken, bootstrap])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            window.set_shadow(true).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
