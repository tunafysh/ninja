use super::scripting::NinjaEngine;
use crate::{manager::ShurikenManager, types::FieldValue};
use anyhow::{Error, Result, bail};
use either::Either;
use shlex::split;
use std::env;
use std::sync::Arc;
use std::{io, path::PathBuf, process::Stdio};
use tokio::process::Command as SubprocessCommand;
use tokio::sync::RwLock;

/// Commands (added ConfigureBlock)
#[derive(Debug, Clone)]
pub enum Command {
    HttpStart(u16),
    Start,
    Stop,
    Select(String),
    Get(String),
    Exit,
    Configure,
    ConfigureBlock(Vec<(String, FieldValue)>),
    Set { key: String, value: FieldValue },
    List,
    ListState,
    Install(PathBuf),
    Toggle(String),
    Execute(PathBuf),
    Help,
    None,
}

// -----------------
// Parsing helpers
// -----------------

// strip single-line comments (// or #)
fn strip_comments(line: &str) -> &str {
    if let Some(i) = line.find("//") {
        &line[..i]
    } else if let Some(i) = line.find('#') {
        &line[..i]
    } else {
        line
    }
}

// detect and parse a single value into FieldValue with basic type detection
fn parse_value(raw: &str) -> FieldValue {
    let v = raw.trim();

    // quoted string support
    if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
        let inner = &v[1..v.len() - 1];
        return FieldValue::String(inner.to_string());
    }

    // boolean
    match v.to_ascii_lowercase().as_str() {
        "true" => return FieldValue::Bool(true),
        "false" => return FieldValue::Bool(false),
        _ => {}
    }

    // integer (i64)
    if let Ok(i) = v.parse::<i64>() {
        return FieldValue::Number(i);
    }

    // fallback: raw string
    FieldValue::String(v.to_string())
}

// parse a single "key = value" text into (key, FieldValue)
fn parse_kv(text: &str) -> Result<Option<(String, FieldValue)>> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }

    if let Some((left, right)) = t.split_once('=') {
        let key = left.trim();
        let val = right.trim();

        if key.is_empty() {
            bail!("Invalid assignment (empty key): `{}`", text);
        }

        // support trailing semicolon being present on the right side
        let val = val.trim_end_matches(';').trim();
        Ok(Some((key.to_string(), parse_value(val))))
    } else {
        // not an assignment (maybe a standalone token) — ignore gracefully
        Ok(None)
    }
}

// collect block content after the '{' and until matching '}'. Supports inline and multiline.
fn collect_block<'a, I>(first_after_brace: &'a str, lines: &mut I) -> Result<String>
where
    I: Iterator<Item = &'a str>,
{
    // If the `first_after_brace` already contains the closing brace on same line
    let trimmed_after = first_after_brace.trim();

    if trimmed_after.ends_with('}') {
        // slice out trailing `}` and return
        let inner = &trimmed_after[..trimmed_after.len() - 1];
        return Ok(inner.trim().to_string());
    }

    // Start with what's after the brace on the first line
    let mut collected = String::new();
    if !trimmed_after.is_empty() {
        collected.push_str(trimmed_after);
        collected.push('\n');
    }

    // gather until we find a line containing a `}` (allow trailing whitespace)
    for next in lines {
        let stripped = strip_comments(next).trim();
        if stripped.ends_with('}') {
            let inner = &stripped[..stripped.len() - 1];
            if !inner.trim().is_empty() {
                collected.push_str(inner.trim());
            }
            return Ok(collected.trim().to_string());
        } else {
            if !stripped.is_empty() {
                collected.push_str(stripped);
                collected.push('\n');
            }
        }
    }

    // If we get here: no closing brace found
    bail!("Missing closing '}}' for block");
}

// ---- Parser: keeps your previous fallback but adds rich configure block handling ----
fn command_parser(script: &str) -> Result<Vec<Command>> {
    let mut commands = Vec::new();

    // iterate over raw lines but keep ownership as &str slices from script.lines()
    let mut lines = script.lines();

    // We need an iterator that allows peeking; we'll manually consume lines as needed
    while let Some(raw_line) = lines.next() {
        // remove comments first
        let no_comment = strip_comments(raw_line);
        let trimmed = no_comment.trim();
        if trimmed.is_empty() {
            continue;
        }

        // ---------- CONFIGURE BLOCK detection ----------
        // handle: configure { ... }  (inline or multiline)
        if trimmed.starts_with("configure") {
            // after the word `configure`, find '{' if present
            if let Some((_, after_brace)) = trimmed.split_once('{') {
                // collect inner content (inline or multiline)
                let block_content = collect_block(after_brace, &mut lines)?;
                // split by semicolons or newlines and parse assignments
                let mut kvs: Vec<(String, FieldValue)> = Vec::new();

                for chunk in block_content.split(|c| c == ';' || c == '\n') {
                    if let Some((k, v)) = parse_kv(chunk)? {
                        kvs.push((k, v));
                    }
                }

                commands.push(Command::ConfigureBlock(kvs));
                continue;
            } else {
                // no brace on this line — treat as `configure` (legacy)
                commands.push(Command::Configure);
                continue;
            }
        }

        // ---------- FALLBACK: use shlex tokenization like before ----------
        // allow tokens with quotes and splitting similar to previous implementation
        if let Some(tokens) = split(trimmed) {
            let tokens: Vec<String> = tokens
                .into_iter()
                .filter(|token| {
                    let token = token.trim();
                    !token.is_empty() && token != "="
                })
                .collect();

            if tokens.is_empty() {
                continue;
            }

            let cmd = match tokens[0].as_str() {
                "http" => Command::HttpStart(tokens[1].parse().unwrap_or(80)),
                "start" => Command::Start,
                "stop" => Command::Stop,
                "select" => {
                    if tokens.len() > 1 {
                        Command::Select(tokens[1].clone())
                    } else {
                        Command::None
                    }
                }
                "help" => Command::Help,
                "get" => {
                    if tokens.len() > 1 {
                        Command::Get(tokens[1].clone())
                    } else {
                        Command::None
                    }
                }
                "set" => {
                    if tokens.len() > 2 {
                        Command::Set {
                            key: tokens[1].clone(),
                            value: parse_value(tokens[2].as_str()),
                        }
                    } else {
                        Command::None
                    }
                }
                "list" => {
                    if tokens.len() > 1 && tokens[1].eq_ignore_ascii_case("state") {
                        Command::ListState
                    } else {
                        Command::List
                    }
                }
                "install" => {
                    if tokens.len() > 1 {
                        Command::Install(PathBuf::from(tokens[1].clone()))
                    } else {
                        Command::None
                    }
                }
                "toggle" => {
                    if tokens.len() > 1 {
                        Command::Toggle(tokens[1].clone())
                    } else {
                        Command::None
                    }
                }
                "execute" => {
                    if tokens.len() > 1 {
                        Command::Execute(PathBuf::from(tokens[1].clone()))
                    } else {
                        Command::None
                    }
                }
                "exit" => Command::Exit,
                "configure" => Command::Configure, // fallback if no {}
                _ => Command::None,
            };

            commands.push(cmd);
        }
    }

    Ok(commands)
}

// ==================
// Helper Function
// ==================

fn locate_ninja_cli() -> Result<PathBuf> {
    let exe_path = env::current_exe()?;
    if let Some(root) = exe_path.parent() {
        let cli_path = if cfg!(windows) {
            root.join("shurikenctl.exe")
        } else {
            root.join("shurikenctl")
        };

        if !cli_path.exists() {
            return Err(Error::msg("No ninja CLI found"));
        }

        Ok(cli_path)
    } else {
        return Err(Error::msg(
            "No parent directory? where and how did you run this? please email me i'm genuinely curious. -- Hannan \"tunafysh\" Smani",
        ));
    }
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
                let path = locate_ninja_cli()?;
                SubprocessCommand::new(path)
                    .arg("api")
                    .arg(port.to_string())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .stdin(Stdio::inherit())
                    .status()
                    .await?;
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

            // simple legacy configure
            Command::Configure => {
                if let Some(name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(name) {
                        let path = &ctx.manager.root_path;
                        shuriken.configure(path.clone()).await.map_err(Error::msg)?;

                        output.push(format!(
                            "Generated configuration for shuriken {} successfully.",
                            &name
                        ));
                    }
                }
            }

            // New: configure block
            Command::ConfigureBlock(kvs) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name)
                        && let Some(cfg) = &mut shuriken.config
                    {
                        let partial_options = cfg.options.get_or_insert_with(Default::default);
                        for (k, v) in kvs {
                            partial_options.insert(k.clone(), v.clone());
                            output.push(format!(
                                "Set {} = {} for {}",
                                k,
                                v.render(),
                                shuriken_name
                            ));
                        }
                    } else {
                        output.push(format!(
                            "No selected shuriken or missing config while applying configure block."
                        ));
                    }
                } else {
                    output.push("No shuriken selected — configure block ignored.".into());
                }
            }

            Command::Help => {
                output.push(
                    "Available commands:
                  http start <port>        - Start the HTTP server
                  select <name>            - Select a shuriken
                  configure                - Generate configuration for the selected shuriken
                  configure { k = v }      - Apply config assignments to the selected shuriken
                  set <key> <value>        - Set a config key for the selected shuriken
                  get <key>                - Get a config key's value
                  toggle <key>             - Toggle a boolean config key
                  start                    - Start the selected shuriken
                  stop                     - Stop the selected shuriken
                  install <path>           - Install a new shuriken from a file
                  list                     - List all shurikens
                  list state               - List shurikens with their states
                  execute <script>         - Run a Ninja script file
                  exit                     - Deselect current shuriken
                  help                     - Show this message"
                        .to_string(),
                );
            }

            // Config commands
            Command::Set { key, value } => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name)
                        && let Some(cfg) = &mut shuriken.config
                    {
                        let cloned_value = value.clone();
                        if let Some(partial_options) = &mut cfg.options {
                            partial_options.insert(key.clone(), FieldValue::from(value.render()));
                        }

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
                        && let Some(options) = &cfg.options
                    {
                        output.push(format!("{:?} = {:?}", key, options.get(&key)));
                    }
                }
            }

            Command::Toggle(key) => {
                if let Some(shuriken_name) = &*ctx.selected.read().await {
                    let mut shurikens = ctx.manager.shurikens.write().await;
                    if let Some(shuriken) = shurikens.get_mut(shuriken_name)
                        && let Some(cfg) = &mut shuriken.config
                        && let Some(options) = &mut cfg.options
                        && let Some(FieldValue::Bool(value)) = options.get_mut(&key)
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
                    .execute_file(&script_path)
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            Command::Install(file_path) => match ctx.manager.install(file_path).await {
                Ok(_) => output.push("Installed successfully".into()),
                Err(e) => output.push(format!("Install failed: {}", e)),
            },

            // Exit the shuriken
            Command::Exit => {
                if ctx.selected.write().await.is_some() {
                    *ctx.selected.write().await = None;
                    output.push("Discarded current shuriken".into());
                } else {
                    output.push("Cannot exit when there's no shuriken to discard.".into());
                }
            }

            // Unsupported
            Command::None => {
                output.push("Invalid or unsupported command.".to_string());
            }
        }
    }

    Ok(output)
}
