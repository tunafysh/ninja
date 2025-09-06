use crate::{manager::ShurikenManager, templater::{infer_value, Value}};
use either::Either;
use std::{io, path::PathBuf};
use std::sync::Arc;
use anyhow::{Result, Error};
use tokio::sync::RwLock;
use ninja_engine::NinjaEngine;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum Command {
    HttpStart(u16),
    Start,
    Stop,
    Select(String),
    Get(String),
    Exit,
    Configure,
    Set { key: String, value: Value },
    List,
    ListState,
    Install(PathBuf),
    Toggle(String),
    Execute(PathBuf),
    None,
}

// ---- Comment-safe remover ----
fn remove_comments(line: &str) -> String {
    let mut in_quotes = false;
    let mut result = String::new();

    for c in line.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes; // toggle quotes
                result.push(c);
            }
            '#' if !in_quotes => {
                // everything after this is a comment
                break;
            }
            _ => result.push(c),
        }
    }

    result
}

// ---- Preprocessor ----
fn preprocessor(script: &str) -> Vec<String> {
    script
        .lines()
        .map(|line| remove_comments(line).trim().replace("=", "")) // remove comments, trim, drop '='
        .filter(|line| !line.is_empty())                            // remove empty lines
        .map(|line| line.to_string())
        .collect()
}

// ---- Command Parser ----
fn command_parser(lines: Vec<String>) -> Result<Vec<Command>> {
    let tokenizer = Regex::new(r#"^(\w+)(?:\s+(\S+))?(?:\s+(.+))?$"#) // allow last arg to include spaces
        .map_err(|e| Error::msg(format!("Failed to create tokenizer: {}", e)))?;

    let mut commands = Vec::new();

    for line in lines {
        if let Some(caps) = tokenizer.captures(&line) {
            let cmd = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let arg1 = caps.get(2).map(|m| m.as_str());
            let arg2 = caps.get(3).map(|m| m.as_str());

            let command = match cmd.to_lowercase().as_str() {
                "http" => match (arg1, arg2) {
                    (Some("start"), Some(port)) => Command::HttpStart(port.parse().unwrap_or(80)),
                    _ => Command::None,
                },
                "start" => Command::Start,
                "stop" => Command::Stop,
                "select" => arg1.map_or(Command::None, |a| Command::Select(a.to_string())),
                "get" => arg1.map_or(Command::None, |a| Command::Get(a.to_string())),
                "set" => match (arg1, arg2) {
                    (Some(k), Some(v)) => Command::Set { key: k.to_string(), value: infer_value(v) },
                    _ => Command::None,
                },
                "list" => match arg1 {
                    Some(a) if a.eq_ignore_ascii_case("state") => Command::ListState,
                    _ => Command::List,
                },
                "install" => arg1.map_or(Command::None, |a| Command::Install(PathBuf::from(a))),
                "toggle" => arg1.map_or(Command::None, |a| Command::Toggle(a.to_string())),
                "execute" => arg1.map_or(Command::None, |a| Command::Execute(PathBuf::from(a))),
                "exit" => Command::Exit,
                "configure" => Command::Configure,
                _ => Command::None,
            };

            commands.push(command);
        }
    }

    Ok(commands)
}


pub struct DslContext {
    pub manager: ShurikenManager,
    pub selected: Arc<RwLock<Option<String>>>,
}

impl DslContext {
    pub fn new(manager: ShurikenManager) -> Self {
        Self {
            manager,
            selected: Arc::new(RwLock::new(None)),
        }
    }
}

pub async fn execute_commands(ctx: &DslContext, script: String) -> Result<Vec<String>> {
    let processed_script = preprocessor(script.as_str());
    let parsed_commands = command_parser(processed_script)?;

    let mut output: Vec<String> = Vec::new();

    for command in parsed_commands {
        match command {
            // HTTP server
            Command::HttpStart(port) => {
                    
                output.push(format!("HTTP server started on port {}", port));
            }

            // Select shuriken
            Command::Select(name) => {
                if ctx.manager.shurikens.read().await.contains_key(&name) {
                    *ctx.selected.write().await = Some(name.clone());
                    output.push(format!("Selected shuriken '{}'", name));
                } else {
                    output.push(format!("No such shuriken: {}", name));
                }
            }

            Command::Configure => {
                if let Some(name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(name) {
                        shuriken.configure().await.map_err(|e| Error::msg(e))?;

                        output.push(format!("Generated configuration for shuriken {} successfully.", &name));
                    } 
                }
            }

            // Config commands
            Command::Set { key, value } => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name) {
                        if let Some(cfg) = &mut shuriken.config {
                            let cloned_value = value.clone();
                            cfg.fields.insert(key.clone(), value);
                            output.push(format!("Set {} = {} for {}", key, cloned_value.render(), shuriken_name));
                        }
                    }
                }
            }

            Command::Get(key) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let shurikens = ctx.manager.shurikens.read().await;
                    if let Some(shuriken) = shurikens.get(shuriken_name) {
                        if let Some(cfg) = &shuriken.config {
                            output.push(format!("{:?} = {:?}", key, cfg.fields.get(&key)));
                        }
                    }
                }
            }

            Command::Toggle(key) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name) {
                        if let Some(cfg) = &mut shuriken.config {
                            if let Some(Value::Bool(value)) = cfg.fields.get_mut(&key) {
                                *value = !*value;
                                output.push(format!("Toggled {} to {}", key, value));
                            }
                        }
                    }
                }
            }

            // Shuriken management
            Command::List => {
                match ctx.manager.list(false).await? {
                    Either::Right(names) => output.push(format!("Shurikens: {:?}", names)),
                    _ => {}
                }
            }
            Command::ListState => {
                match ctx.manager.list(true).await? {
                    Either::Left(states) => {
                        for (name, state) in states {
                            output.push(format!("{} -> {:?}", name, state));
                        }
                    }
                    _ => {}
                }
            }

            Command::Start => {
                if let Some(name) = &*ctx.selected.read().await {
                    match ctx.manager.start(&name).await {
                        Ok(_) => output.push(format!("Started {}", name)),
                        Err(e) => output.push(format!("Error: {}", e)),
                    }
                }
            }
            Command::Stop => {
                if let Some(name) = &*ctx.selected.read().await {

                    match ctx.manager.stop(&name).await {
                        Ok(_) => output.push(format!("Stopped {}", name)),
                        Err(e) => output.push(format!("Error: {}", e)),
                    }
                }
            }

            Command::Execute(script_path) => {
                let engine = NinjaEngine::new()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                engine.execute_file(script_path.display().to_string().as_str())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            }
            Command::Install(file_path) => {
                match ctx.manager.install(file_path).await {
                    Ok(_) => output.push("Installed successfully".into()),
                    Err(e) => output.push(format!("Install failed: {}", e)),
                }
            }

            // Exit the shuriken
            Command::Exit => {
                *ctx.selected.write().await = None;
                output.push("Deselected current shuriken".into());
            }

            // Unsupported
            Command::None => {
                output.push(format!("Invalid or unsupported command:"));
            }
        }
    }

    Ok(output)
}
