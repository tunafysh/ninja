use log::{error, info, warn};
use ninja::{common::structs::NoopReporter, manager::ShurikenManager};
use std::collections::HashMap;
use tauri::State;
use tokio::sync::Mutex;
use url::Url;

pub async fn handle_shurikenctl(url: &str, manager: State<'_, Mutex<ShurikenManager>>) {
    let url = url.to_string();
    let manager = manager.lock().await;
    let engine = manager.new_dsl();
    let parsed = match Url::parse(&url) {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid shurikenctl URL '{}': {}", url, e);
            return;
        }
    };

    println!("Parsed URL: {:?}", parsed);

    let command = parsed.host_str().unwrap_or("").trim_start_matches('/');
    let query: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

    println!("Command: {}", command);
    println!("Query: {:?}", query);

    match command {
        "install" => {
            if let Some(pkg) = query.get("pkg") {
                info!("Installing Shuriken: {}", pkg);
                let reporter = NoopReporter {};
                if let Err(e) = manager.install(&pkg, reporter).await {
                    error!("Failed to install '{}': {}", pkg, e);
                }
            } else {
                warn!("install command missing 'pkg' parameter");
            }
        }
        "start" => {
            if let Some(shuriken) = query.get("shuriken") {
                info!("Starting Shuriken: {}", shuriken);
                if let Err(e) = manager.start(shuriken).await {
                    error!("Failed to start '{}': {}", shuriken, e);
                }
            } else {
                warn!("start command missing 'shuriken' parameter");
            }
        }
        "stop" => {
            if let Some(shuriken) = query.get("shuriken") {
                info!("Stopping Shuriken: {}", shuriken);
                if let Err(e) = manager.stop(shuriken).await {
                    error!("Failed to stop '{}': {}", shuriken, e);
                }
            } else {
                warn!("stop command missing 'shuriken' parameter");
            }
        }
        "execute" => {
            if let Some(cmd) = query.get("cmd") {
                info!("Executing DSL command: {}", cmd);
                if let Err(e) = engine.execute(cmd.clone()).await {
                    error!("Failed to execute DSL command '{}': {}", cmd, e);
                }
            } else {
                warn!("execute command missing 'cmd' parameter");
            }
        }
        "http" => {
            if let Some(port) = query.get("port") {
                info!("Starting HTTP service on port {}", port);
                if let Err(e) = engine.execute(format!("http start {}", port)).await {
                    error!("Failed to start HTTP API on port {}: {}", port, e);
                }
            } else {
                warn!("http command missing 'port' parameter");
            }
        }
        "refresh" => {
            info!("Refreshing Shuriken list...");
            if let Err(e) = manager.refresh().await {
                error!("Failed to refresh Shurikens: {}", e);
            }
        }
        _ => {
            warn!("Unknown shurikenctl command: {}", command);
        }
    }
}
