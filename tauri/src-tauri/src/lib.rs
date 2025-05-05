use std::path::Path;
use glob::glob;
use tauri::Manager;
use std::collections::HashMap;
use log::info;
use std::fs;

mod shuriken;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/\
#[tauri::command]
fn bootstrap() -> Vec<shuriken::Shuriken> {
    info!("Bootstrapping...");
    
    if !fs::exists("shurikens").expect("Cannot read fs") {
        fs::create_dir("shurikens").expect("Failed to create shuriken directory");
    }
    
    let shurikens = shuriken::ShurikenManager::discover("shurikens/manifest.toml").unwrap();

    info!("Bootstrapped {} shuriken(s)", shurikens.len());
    shurikens
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .filter(|metadata| metadata.target() != "tao")
            .build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![control_shuriken, bootstrap])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            window.set_shadow(true).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
