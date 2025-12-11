use ninja::manager::ShurikenManager;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use url::Url;
mod commands;
use commands::*;
use dirs_next::home_dir;
use std::fs;
use tokio::sync::Mutex;

mod link_parser;

fn is_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::info!("Starting Tauri application...");

    let mut builder = tauri::Builder::default()
        .setup(|app| {
            // Initialize ShurikenManager
            let manager = Mutex::new(
                tauri::async_runtime::block_on(ShurikenManager::new())
                    .expect("Failed to spawn a shuriken manager"),
            );
            app.manage(manager);
            {
                let resource_dir = app.path().resource_dir()?;
                let cheatsheet = resource_dir.join("cheatsheet.md");
                let docs = resource_dir.join("docs.md");
                let important_target_path = home_dir().unwrap()
                    .join(".ninja")
                    .join("assets");
                let docs_target_path = home_dir().unwrap()
                    .join(".ninja")
                    .join("docs");
                let important_file = important_target_path.join("coconut.jpg");
                if !important_target_path.exists() {
                    fs::create_dir_all(&important_target_path).expect("Failed to create docs directory");
                    fs::copy(&important_file, important_target_path.join("coconut.jpg")).expect("Failed to copy coconut.jpg");
                }
                if !docs_target_path.exists() {
                    fs::create_dir_all(&docs_target_path).expect("Failed to create docs directory");
                    fs::copy(&docs, docs_target_path.join("docs.md")).expect("Failed to copy cheatsheet.md");
                    fs::copy(&cheatsheet, docs_target_path.join("cheatsheet.md")).expect("Failed to copy cheatsheet.md");
                }
                if !important_file.exists() {
                    app.dialog()
                        .message("Required file 'coconut.jpg' is missing.\nNinja will not run without it.\nIf you think this is a mistake — reinstall or restore the file.\n(Yes — the coconut is important.)")
                        .kind(MessageDialogKind::Error)
                        .title("Source Engine Error")
                        .blocking_show();
                }
            }
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                app.deep_link().register_all()?;
            }

            #[cfg(target_os = "macos")]
            {
                let window = app.get_webview_window("main").unwrap();
                window.set_shadow(true).unwrap();
            }

            Ok(())
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Clone the handle, which is owned
            let app_handle = app.clone();

            if is_url(argv[1].as_str()) {
                tauri::async_runtime::spawn(async move {
                    let manager_state = app_handle.state::<Mutex<ShurikenManager>>().clone();
                    // Now you have `app_handle` too if you need to emit or do things
                    link_parser::handle_shurikenctl(&argv[1], manager_state.clone()).await;
                });
            } else {
                let metadata = open_shuriken(argv[1].to_string().clone()).unwrap();
                app.emit("view_local_shuriken", (metadata, argv[1].to_string()))
                    .unwrap();
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
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
            execute_dsl,
            configure_shuriken,
            refresh_shurikens,
            developer_mode,
            open_dir,
            save_config,
            get_projects,
            get_project_readme,
            open_shuriken,
            install_shuriken,
            backup_restore,
            backup_now
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
