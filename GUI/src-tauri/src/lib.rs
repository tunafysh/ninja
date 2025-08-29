use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

mod commands;
use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::info!("Starting Tauri application...");
    let mut builder = tauri::Builder::default();
        #[cfg(desktop)]
        {
            builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let _ = app.dialog()
                .message(format!("The deep-link works with the arguments {} was executed successfully", argv.join(", ")))
                .kind(MessageDialogKind::Info)
                .title("The deep-link works")
                .blocking_show();
            }));
        }
        builder = builder
        .plugin(tauri_plugin_deep_link::init())
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
            enable_mcp,
            disable_mcp
        ])
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            #[cfg(target_os = "macos")]
            {
                let window = _app.get_webview_window("main").unwrap();
                window.set_shadow(true).unwrap();
            };
            Ok(())
        });

        builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}