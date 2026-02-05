use ninja::manager::ShurikenManager;
use tauri::{Emitter, Manager, menu::Menu};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use url::Url;
mod commands;
use commands::*;
use dirs_next::home_dir;
use std::{fs, path::Path};
use tokio::sync::Mutex;

mod link_parser;

fn is_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}

fn create_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    use tauri::menu::MenuBuilder;

    let menu = MenuBuilder::new(app)
        .build()?;

    Ok(menu)
}

fn ensure_assets_exist(
    resource_dir: &Path,
    home_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let important_target_path = home_dir.join(".ninja/assets");
    let docs_target_path = home_dir.join(".ninja/docs");

    // Try creating directories
    if let Err(e) = fs::create_dir_all(&important_target_path) {
        log::warn!("Could not create assets directory: {e}");
    }
    if let Err(e) = fs::create_dir_all(&docs_target_path) {
        log::warn!("Could not create docs directory: {e}");
    }

    // Copy critical file safely
    let coconut_file = resource_dir.join("coconut.jpg");
    let target_coconut = important_target_path.join("coconut.jpg");
    if !target_coconut.exists() {
        if coconut_file.exists() {
            if let Err(e) = fs::copy(&coconut_file, &target_coconut) {
                log::warn!("Failed to copy coconut.jpg: {e}");
            }
        } else {
            log::error!("Critical file 'coconut.jpg' is missing in resources");
        }
    }

    // Copy docs files
    for (src, dst_name) in &[
        (resource_dir.join("docs.md"), "docs.md"),
        (resource_dir.join("cheatsheet.md"), "cheatsheet.md"),
    ] {
        let dst = docs_target_path.join(dst_name);
        if !dst.exists() && src.exists() {
            if let Err(e) = fs::copy(src, &dst) {
                log::warn!("Failed to copy {dst_name}: {e}");
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log::info!("Starting Tauri application...");

    let mut builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let menu = create_menu(app.handle())?;
            app.set_menu(menu)?;

            let manager = Mutex::new(
                tauri::async_runtime::block_on(ShurikenManager::new())
                    .expect("Failed to spawn a shuriken manager"),
            );
            app.manage(manager);

            let resource_dir = app.path().resource_dir()?;
            let home = home_dir().ok_or("Cannot determine home directory")?;

            if let Err(e) = ensure_assets_exist(&resource_dir, &home) {
                app.dialog()
                    .message(format!("{e}\nReinstall or restore the file."))
                    .kind(MessageDialogKind::Error)
                    .title("Source Engine Error")
                    .blocking_show();
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
            backup_now,
            lockpick_shuriken
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
