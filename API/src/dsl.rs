use crate::manager::ShurikenManager;
use either::Either;
use chumsky::prelude::*;
use std::{io, path::PathBuf};
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::RwLock;
use ninja_engine::{InputType, NinjaEngine};

fn infer_input_type(value: &str) -> InputType {
    let val = value.trim();

    if let Ok(n) = val.parse::<i64>() {
        InputType::Number {
            default: None,
            min: None,
            max: None,
            value: n,
        }
    } else if val.eq_ignore_ascii_case("true") {
        InputType::Boolean {
            default: None,
            value: true,
        }
    } else if val.eq_ignore_ascii_case("false") {
        InputType::Boolean {
            default: None,
            value: false,
        }
    } else if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
        InputType::Text {
            default: None,
            regex: None,
            value: val[1..val.len() - 1].to_string(),
        }
    } else {
        InputType::Text {
            default: None,
            regex: None,
            value: val.to_string(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub args: Args,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Args {
    None,
    Single(String),
    Double { key: String, value: String },
}

fn args_parser() -> ! {
    // Quoted string: "hello"
    todo!()
}

fn command_parser() -> ! {
    todo!()
}

fn commands_parser() -> ! {
    todo!()
}

fn parse_commands(input: &str) -> Vec<Command> {
    vec![]
}

pub struct DslContext {
    pub manager: ShurikenManager,
    pub selected: Arc<RwLock<Option<String>>>,
    pub http_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl DslContext {
    pub fn new(manager: ShurikenManager) -> Self {
        Self {
            manager,
            selected: Arc::new(RwLock::new(None)),
            http_handle: Arc::new(RwLock::new(None)),
        }
    }
}

pub async fn execute_commands(ctx: &DslContext, script: String) -> Result<String, io::Error> {
    let parsed_commands = parse_commands(&script);

    let mut output = Vec::new();

    for command in parsed_commands {
        match (command.name.as_str(), command.args.clone()) {
            // HTTP server
            ("http", Args::Double { key, value }) if key == "start" => {
                if let Ok(port) = value.parse::<u16>() {
                    
                    output.push(format!("HTTP server started on port {}", port));
                } else {
                    output.push(format!("Invalid port: {}", value));
                }
            }
            ("http", Args::Single(ref s)) if s == "stop" => {
                let mut handle_lock = ctx.http_handle.write().await;
                if let Some(handle) = handle_lock.take() {
                    handle.abort();
                    output.push("HTTP server stopped".into());
                } else {
                    output.push("HTTP server not running".into());
                }
            }

            // Select shuriken
            ("select", Args::Single(name)) => {
                if ctx.manager.shurikens.read().await.contains_key(&name) {
                    *ctx.selected.write().await = Some(name.clone());
                    output.push(format!("Selected shuriken '{}'", name));
                } else {
                    output.push(format!("No such shuriken: {}", name));
                }
            }

            // Config commands
            ("set", Args::Double { key, value }) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name) {
                        if let Some(cfg) = &mut shuriken.config {
                            cfg.insert(key.clone(), infer_input_type(&value));
                            output.push(format!("Set {} = {} for {}", key, value, shuriken_name));
                        }
                    }
                }
            }

            ("get", Args::Single(key)) | ("show", Args::Single(key)) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let shurikens = ctx.manager.shurikens.read().await;
                    if let Some(shuriken) = shurikens.get(shuriken_name) {
                        if let Some(cfg) = &shuriken.config {
                            output.push(format!("{:?} = {:?}", key, cfg.get(&key)));
                        }
                    }
                }
            }

            ("toggle", Args::Single(key)) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name) {
                        if let Some(cfg) = &mut shuriken.config {
                            if let Some(InputType::Boolean { value, .. }) = cfg.get_mut(&key) {
                                *value = !*value;
                                output.push(format!("Toggled {} to {}", key, value));
                            }
                        }
                    }
                }
            }

            // Shuriken management
            ("list", Args::None) => {
                match ctx.manager.list(false).await? {
                    Either::Right(names) => output.push(format!("Shurikens: {:?}", names)),
                    _ => {}
                }
            }
            ("list", Args::Single(arg)) if arg == "state" => {
                match ctx.manager.list(true).await? {
                    Either::Left(states) => {
                        for (name, state) in states {
                            output.push(format!("{} -> {:?}", name, state));
                        }
                    }
                    _ => {}
                }
            }
            ("start", Args::Single(name)) => {
                match ctx.manager.start(&name).await {
                    Ok(_) => output.push(format!("Started {}", name)),
                    Err(e) => output.push(format!("Error: {}", e)),
                }
            }
            ("stop", Args::Single(name)) => {
                match ctx.manager.stop(&name).await {
                    Ok(_) => output.push(format!("Stopped {}", name)),
                    Err(e) => output.push(format!("Error: {}", e)),
                }
            }
            ("execute", Args::Single(script_path)) => {
                let engine = NinjaEngine::new()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                engine.execute_file(script_path.as_str())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            }
            ("install", Args::Single(file_path)) => {
                let pb = PathBuf::from(file_path);
                match ctx.manager.install(pb).await {
                    Ok(_) => output.push("Installed successfully".into()),
                    Err(e) => output.push(format!("Install failed: {}", e)),
                }
            }

            // Exit the shuriken
            ("exit", Args::None) => {
                *ctx.selected.write().await = None;
                output.push("Deselected current shuriken".into());
            }

            // Unsupported
            (name, args) => {
                output.push(format!("Invalid or unsupported command: {} {:?}", name, args));
            }
        }
    }

    Ok(output.join("\n"))
}
