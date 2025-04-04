use tauri::{command, State};
use std::sync::Mutex;
use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/\

fn execute_command(command: String, state: State<'_, AppState>) -> Result<String, String> {
    let current_dir = state.current_dir.lock().unwrap().clone();
    
    // Split the command into program and arguments
    let mut parts = command.split_whitespace();
    let program = parts.next().unwrap_or("");
    let args: Vec<&str> = parts.collect();
    
    // Handle cd command specially
    if program == "cd" {
        if let Some(dir) = args.first() {
            let path = PathBuf::from(dir);
            let mut new_dir = current_dir.clone();
            
            if path.is_absolute() {
                new_dir = path;
            } else {
                new_dir.push(path);
            }
            
            // Update the current directory
            if new_dir.exists() {
                *state.current_dir.lock().unwrap() = new_dir.clone();
                return Ok(format!("Changed directory to {}", new_dir.display()));
            } else {
                return Err(format!("Directory not found: {}", dir));
            }
        } else {
            // cd without arguments goes to home directory
            if let Some(home) = dirs::home_dir() {
                *state.current_dir.lock().unwrap() = home.clone();
                return Ok(format!("Changed directory to {}", home.display()));
            } else {
                return Err("Could not determine home directory".to_string());
            }
        }
    }
    
    // Execute the command
    let output = Command::new(program)
        .args(args)
        .current_dir(&current_dir)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    // Combine stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    if !stderr.is_empty() {
        return Err(stderr);
    }
    
    Ok(stdout)
}

#[command]
fn get_current_dir(state: State<'_, AppState>) -> String {
    state.current_dir.lock().unwrap().display().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_current_dir, execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
