use crate::{
    manager::ShurikenManager,
    templater::{Value, infer_value},
};
use anyhow::{Error, Result};
use either::Either;
use ninja_engine::NinjaEngine;
use shlex::split;
use std::sync::Arc;
use std::{io, path::PathBuf};
use tokio::sync::RwLock;

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

// ---- Preprocessor ----
fn parser(script: &str) -> Vec<Vec<String>> {
    let lines: Vec<&str> = script.split('\n').collect();

    let mut output: Vec<Vec<String>> = Vec::new();

    for line in lines {
        if let Some(tokens) = split(line) {
            let tokens: Vec<String> = tokens
                .into_iter()
                .filter(|token| {
                    let token = token.trim();
                    !token.is_empty() && token != "="
                })
                .collect();

            if !tokens.is_empty() {
                output.push(tokens)
            }
        }
    }

    output
}

// ---- Command Parser ----
fn command_parser(script: &str) -> Result<Vec<Command>> {
    let tokens = parser(script);

    let mut commands = Vec::new();

    for token in tokens {
        let command = match token[0].as_str() {
            "http" => match (token[1].as_str(), &token[2]) {
                ("start", port) => Command::HttpStart(port.parse().unwrap_or(80)),
                _ => Command::None,
            },
            "start" => Command::Start,
            "stop" => Command::Stop,
            "select" => {
                if !token[1].is_empty() {
                    Command::Select(token[1].clone())
                } else {
                    Command::None
                }
            }
            "get" => {
                if !token[1].is_empty() {
                    Command::Get(token[1].clone())
                } else {
                    Command::None
                }
            }
            "set" => {
                let (k, v) = (&token[1], &token[2]);
                Command::Set {
                    key: k.clone(),
                    value: infer_value(v),
                }
            }
            "list" => match token[1].clone() {
                a if a.eq_ignore_ascii_case("state") => Command::ListState,
                _ => Command::List,
            },
            "install" => {
                if !token[1].is_empty() {
                    Command::Install(PathBuf::from(token[1].clone()))
                } else {
                    Command::None
                }
            }
            "toggle" => {
                if !token[1].is_empty() {
                    Command::Toggle(token[1].clone())
                } else {
                    Command::None
                }
            }
            "execute" => {
                if !token[1].is_empty() {
                    Command::Execute(PathBuf::from(token[1].clone()))
                } else {
                    Command::None
                }
            }
            "exit" => Command::Exit,
            "configure" => Command::Configure,
            _ => Command::None,
        };

        commands.push(command);
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
    let parsed_commands = command_parser(script.as_str())?;

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
                        shuriken.configure().await.map_err(Error::msg)?;

                        output.push(format!(
                            "Generated configuration for shuriken {} successfully.",
                            &name
                        ));
                    }
                }
            }

            // Config commands
            Command::Set { key, value } => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name)
                        && let Some(cfg) = &mut shuriken.config
                    {
                        let cloned_value = value.clone();
                        cfg.fields.insert(key.clone(), value);
                        output.push(format!(
                            "Set {} = {} for {}",
                            key,
                            cloned_value.render(),
                            shuriken_name
                        ));
                    }
                }
            }

            Command::Get(key) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let shurikens = ctx.manager.shurikens.read().await;
                    if let Some(shuriken) = shurikens.get(shuriken_name)
                        && let Some(cfg) = &shuriken.config
                    {
                        output.push(format!("{:?} = {:?}", key, cfg.fields.get(&key)));
                    }
                }
            }

            Command::Toggle(key) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name)
                        && let Some(cfg) = &mut shuriken.config
                        && let Some(Value::Bool(value)) = cfg.fields.get_mut(&key)
                    {
                        *value = !*value;
                        output.push(format!("Toggled {} to {}", key, value));
                    }
                }
            }

            // Shuriken management
            Command::List => {
                if let Either::Right(names) = ctx.manager.list(false).await? {
                    output.push(format!("Shurikens: {:?}", names))
                }
            }
            Command::ListState => {
                if let Either::Left(states) = ctx.manager.list(true).await? {
                    for (name, state) in states {
                        output.push(format!("{} -> {:?}", name, state));
                    }
                }
            }

            Command::Start => {
                if let Some(name) = &*ctx.selected.read().await {
                    match ctx.manager.start(name).await {
                        Ok(_) => output.push(format!("Started {}", name)),
                        Err(e) => output.push(format!("Error: {}", e)),
                    }
                }
            }
            Command::Stop => {
                if let Some(name) = &*ctx.selected.read().await {
                    match ctx.manager.stop(name).await {
                        Ok(_) => output.push(format!("Stopped {}", name)),
                        Err(e) => output.push(format!("Error: {}", e)),
                    }
                }
            }

            Command::Execute(script_path) => {
                let engine = NinjaEngine::new().map_err(|e| io::Error::other(e.to_string()))?;
                engine
                    .execute_file(script_path.display().to_string().as_str())
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            Command::Install(file_path) => match ctx.manager.install(file_path).await {
                Ok(_) => output.push("Installed successfully".into()),
                Err(e) => output.push(format!("Install failed: {}", e)),
            },

            // Exit the shuriken
            Command::Exit => {
                *ctx.selected.write().await = None;
                output.push("Deselected current shuriken".into());
            }

            // Unsupported
            Command::None => {
                output.push("Invalid or unsupported command.".to_string());
            }
        }
    }

    Ok(output)
}
