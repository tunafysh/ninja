use tauri::Manager;

mod commands;
use commands::*;

#[tauri::command]
async fn toggle_armory_window(app: tauri::AppHandle) {
    let label = "armory";

    if let Some(window) = app.get_webview_window(label) {
        // If the window already exists, close it
        let _ = window.close();
    } else {
        // If not, create it
        let _ = tauri::WebviewWindowBuilder::new(
            &app,
            label,
            tauri::WebviewUrl::App("/armory".into()), // or a specific route
        )
        .title("Armory")
        .inner_size(912.0, 513.0)
        .resizable(false)
        .decorations(false)
        .fullscreen(false)
        .build();
    }
}

#[tauri::command]
async fn toggle_forge_window(app: tauri::AppHandle) {
    let label = "forge";

    if let Some(window) = app.get_webview_window(label) {
        // If the window already exists, close it
        let _ = window.close();
    } else {
        // If not, create it
        let _ = tauri::WebviewWindowBuilder::new(
            &app,
            label,
            tauri::WebviewUrl::App("/forge".into()), // or a specific route
        )
        .title("Forge")
        .inner_size(912.0, 513.0)
        .resizable(false)
        .decorations(false)
        .fullscreen(false)
        .build();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::info!("Starting Tauri application...");
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|_app, argv, _cwd| {
            println!("got arg: {}", argv[1])
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .filter(|metadata| !metadata.target().starts_with("tao"))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_shuriken,
            stop_shuriken,
            get_all_shurikens,
            get_running_shurikens,
            get_shuriken_config,
            toggle_armory_window,
            toggle_forge_window,
            enable_mcp,
            disable_mcp
        ])
        .setup(|app| {
            use tauri_plugin_deep_link::DeepLinkExt;
            app.deep_link().register_all()?;

            #[cfg(target_os = "macos")]
            {
                let window = _app.get_webview_window("main").unwrap();
                window.set_shadow(true).unwrap();
            };
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
