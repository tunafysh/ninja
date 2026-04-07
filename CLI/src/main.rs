use ::log::info;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use ninja::{
    VERSION,
    common::{
        config::{
            ShurikenReference, find_shuriken_in_registries, get_shuriken_info, resolve_shuriken_url,
        },
        registry::ArmoryItem,
        types::{ArmoryMetadata, ShurikenState},
    },
    manager::ShurikenManager,
    shuriken::{Shuriken, ShurikenConfig, ShurikenMetadata},
};
use ninja_http::server;
use ninja_mcp::server as mcpserver;
use owo_colors::OwoColorize;
use std::{
    env,
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
    process::exit,
};

use tokio::fs;

mod log;
use log::setup_logger;

mod repl;
use repl::repl_mode;

mod prompts;
use prompts::{collect_forge_metadata, collect_new_shuriken_input};

#[derive(Parser)]
#[command(name = "ninja")]
#[command(version = VERSION)]
#[command(about = "shurikenctl - The command line version of Ninja")]
struct NinjaCli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, hide = true)]
    pub mcp: bool,

    #[arg(long)]
    pub repl: bool,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a shuriken
    Start(StartArgs),
    /// Stop a shuriken
    Stop(StopArgs),
    /// Run a script using the Ninja Runtime
    Run(RunArgs),
    /// List shuriken services with their statuses
    List,
    /// Generate a new shuriken with specified manifest
    New,
    /// Configure a shuriken
    Configure(ConfigureArgs),
    /// Lockpick a shuriken (remove the .lck file, dangerous/use with caution)
    Lockpick(LockpickArgs),
    /// Start up the HTTP API with a specified port (optional but recommended).
    Api(ApiArgs),
    /// Install a shuriken
    Install(InstallArgs),
    /// Forge a new shuriken (.shuriken file) from a local one
    Forge(ForgeArgs),
    /// Remove a shuriken (uninstall it completely)
    Remove(RemoveArgs),
    /// Manage registries and get shuriken information
    Registry(RegistryArgs),
}

#[derive(Args)]
pub struct ApiArgs {
    /// The port for the HTTP api to use
    pub port: u16,
}

#[derive(Args)]
pub struct StartArgs {
    /// The name of the shuriken to start
    pub shuriken: String,
}

#[derive(Args)]
pub struct StopArgs {
    /// The name of the shuriken to stop
    pub shuriken: String,
}

#[derive(Args)]
pub struct ConfigureArgs {
    /// The name of the shuriken to configure
    pub shuriken: String,
}

#[derive(Args)]
pub struct LockpickArgs {
    /// The name of the shuriken to lockpick
    pub shuriken: String,
}

#[derive(Args)]
pub struct RunArgs {
    /// The path of the file or snippet of script to run
    #[arg(name = "file/script")]
    pub file_script: Option<String>,
}

#[derive(Args)]
pub struct ListArgs {
    /// Show all shurikens and their statuses
    #[arg(short = 'f', long)]
    pub full: bool,
}

#[derive(Args)]
pub struct InstallArgs {
    /// The path of the .shuriken file to install
    pub path: PathBuf,
}

#[derive(Args)]
pub struct ForgeArgs {
    /// The path of the files to forge a shuriken (with the .ninja folder and everything)
    pub path: PathBuf,
    /// optional path to something like forge-options.json to skip inputs (CI friendly)
    #[arg(short = 'c', long)]
    pub options: Option<PathBuf>,
    /// optional output path
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// The name of the shuriken to remove/uninstall, it's the same thing
    pub shuriken: String,
}

#[derive(Subcommand)]
pub enum RegistrySubcommands {
    /// Get information about a shuriken from registries
    Get(RegistryGetArgs),
    /// Install a shuriken from the registries using its reference (registry:shuriken)
    Install(RegistryInstallArgs),
}

#[derive(Args)]
pub struct RegistryArgs {
    #[command(subcommand)]
    pub subcommand: RegistrySubcommands,
}

#[derive(Args)]
pub struct RegistryGetArgs {
    /// The shuriken reference in format "registry:shuriken"
    pub reference: String,
}

#[derive(Args)]
pub struct RegistryInstallArgs {
    /// The shuriken reference in format "registry:shuriken"
    pub reference: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = NinjaCli::parse();

    // Initialize logger
    setup_logger(args.verbose.into())?;

    if args.repl {
        repl_mode().await?;
        exit(0)
    }

    if args.mcp {
        info!("Starting up as an MCP server.");
        mcpserver().await?;
        return Ok(());
    }

    info!("Initializing service manager...");
    let manager = ShurikenManager::new()
        .await
        .map_err(|e| format!("Failed to initialize service manager: {}", e))?;

    match args.command {
        Some(Commands::Start(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;

            println!("Starting shuriken {}...\n", shuriken_name);
            // Use the actual name from manifest, not service-name
            match manager.start(shuriken_name.as_str()).await {
                Ok(_) => println!("\nStarted shuriken '{}'", shuriken_name.green()),
                Err(e) => eprintln!(
                    "{}",
                    format!("Failed to start shuriken '{}': {}", shuriken_name, e).red()
                ),
            }
        }
        Some(Commands::Stop(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;

            println!("Stopping shuriken {}...\n", shuriken_name);
            // Use the actual name from manifest, not service-name
            match manager.stop(shuriken_name.as_str()).await {
                Ok(_) => println!("\nStopped shuriken '{}'", shuriken_name.red()),
                Err(e) => eprintln!(
                    "{}",
                    format!("Failed to stop shuriken '{}': {}", shuriken_name, e).red()
                ),
            }
        }
        Some(Commands::List) => {
            let partial_shurikens = manager.list(true).await?.left();
            if let Some(shurikens) = partial_shurikens {
                println!("{}", "Shurikens:\n".blue().bold());
                for (name, state) in shurikens {
                    if state == ShurikenState::Running {
                        println!("{} {}", name, "running".green());
                    } else {
                        println!("{} {}", name, "stopped".red());
                    }
                }
            } else {
                eprintln!("Failed to list shurikens, None returned.")
            }

            println!(); // for styling purposes
        }
        Some(Commands::Run(script_args)) => {
            let file_arg = script_args.file_script.ok_or("path argument is empty")?;
            let content = file_arg.as_str();
            let path = PathBuf::from(content);
            if path.exists() {
                match manager
                    .engine
                    .lock()
                    .await
                    .execute_file(&path, None, Some(manager.clone()))
                {
                    Ok(_) => exit(0),
                    Err(e) => eprintln!("Error: {}", e),
                }
            } else {
                match manager.engine.lock().await.execute(
                    content,
                    Some(&manager.root_path),
                    Some(manager.clone()),
                ) {
                    Ok(_) => exit(0),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Some(Commands::New) => {
            let input = collect_new_shuriken_input()?;
            let shuriken_name = input.name.clone();

            println!(
                "{}",
                format!("Generating manifest for '{}'", shuriken_name).bold()
            );

            let manifest = Shuriken {
                metadata: ShurikenMetadata {
                    name: input.name.clone(),
                    id: input.id,
                    version: input.version,
                    script_path: Some(input.script_path.clone()),
                    import_script: None,
                    shuriken_type: input.shuriken_type,
                },
                config: input.config_path.map(|path| ShurikenConfig {
                    config_path: path,
                    options: None,
                }),
                logs: None,
                tools: None,
            };

            create_dir_all(format!("shurikens/{}/.ninja", shuriken_name)).unwrap_or_else(|_| {
                eprintln!(
                    "Failed to create directory for shuriken '{}'",
                    shuriken_name
                );
                exit(1);
            });

            env::set_current_dir(format!("shurikens/{}/.ninja", shuriken_name))?;

            if let Some(opts) = input.options {
                let serialized_options = toml::ser::to_string_pretty(&opts)?;
                fs::write("config.tmpl", "").await?;
                fs::write("options.toml", serialized_options).await?;
            }

            let manifest_path = PathBuf::from("manifest.toml");
            let mut file = File::create(&manifest_path).unwrap_or_else(|_| {
                eprintln!(
                    "Failed to create manifest file for shuriken '{}'",
                    shuriken_name
                );
                exit(1);
            });

            if let Some(parent) = input.script_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(
                    &input.script_path,
                    "function start()\n\t-- Start procedure goes here\nend\n\nfunction stop()\n\t-- Stop procedure goes here\nend",
                )
                .await?;

            let toml_content = toml::to_string(&manifest).unwrap_or_else(|_| {
                eprintln!(
                    "Failed to serialize manifest for shuriken '{}'",
                    shuriken_name
                );
                exit(1);
            });

            file.write_all(toml_content.as_bytes()).unwrap_or_else(|_| {
                eprintln!(
                    "Failed to write manifest file for shuriken '{}'",
                    shuriken_name
                );
                exit(1);
            });

            env::set_current_dir(&manager.root_path)?;

            println!("Manifest for '{}' generated successfully!", shuriken_name);
        }
        Some(Commands::Configure(args)) => {
            info!("Configuring shuriken {}", args.shuriken);
            manager.configure(&args.shuriken).await?;
        }
        Some(Commands::Lockpick(args)) => {
            info!("Lockpicking shuriken {}", args.shuriken);
            manager.lockpick(&args.shuriken).await?;
        }

        Some(Commands::Api(args)) => {
            info!("Starting API endpoint with port {}", args.port);
            server(args.port).await?;
        }
        Some(Commands::Install(args)) => {
            info!("Installing a shuriken");
            manager.install(&args.path).await?;
        }
        Some(Commands::Forge(args)) => {
            use serde_json::from_str;
            use tokio::fs;

            if let Some(config_path) = args.options {
                // --- Load metadata from config file ---
                let serialized_metadata = fs::read_to_string(&config_path).await?;
                let metadata: ArmoryMetadata = from_str(&serialized_metadata)?;

                println!("{}", "Creating shuriken...".bold());

                // No need to manually create "blacksmith" here,
                // `forge` already ensures the directory exists.
                manager.forge(metadata, args.path, args.output).await?;
            } else {
                let metadata = collect_forge_metadata()?;

                println!("{}", "Creating shuriken...".bold());
                manager.forge(metadata, args.path, args.output).await?;
            }
        }
        Some(Commands::Remove(args)) => {
            manager.remove(&args.shuriken).await?;
        }
        Some(Commands::Registry(registry_args)) => {
            let config = manager.config.read().await;
            match registry_args.subcommand {
                RegistrySubcommands::Get(get_args) => {
                    info!(
                        "Fetching info for shuriken reference: {}",
                        get_args.reference
                    );

                    let reference = ShurikenReference::parse(&get_args.reference)?;
                    match get_shuriken_info(&config.registries, &reference).await {
                        Ok(info) => {
                            println!("{}", serde_json::to_string_pretty(&info)?);
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Failed to get shuriken info: {}", e).red());
                            exit(1);
                        }
                    }
                }
                RegistrySubcommands::Install(install_args) => {
                    info!(
                        "Installing shuriken from registry with reference: {}",
                        install_args.reference
                    );
                    // Similar to Get, but we also resolve the URL and then call manager.install_url
                    let reference = ShurikenReference::parse(&install_args.reference)?;
                    match find_shuriken_in_registries(&config.registries, &reference).await {
                        Ok((shuriken, registry_name)) => {
                            let registry_url =
                                config.registries.get(&registry_name).ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Registry URL not found for '{}'",
                                        registry_name
                                    )
                                })?;
                            let shuriken_url = match shuriken {
                                ArmoryItem::Shuriken { url, .. } => url,
                                ArmoryItem::Bundle { shurikens, .. } => {
                                    for shuriken_ref in shurikens {
                                        let item_ref = ShurikenReference {
                                            registry: registry_name.clone(),
                                            shuriken: shuriken_ref.clone(),
                                        };
                                        match find_shuriken_in_registries(
                                            &config.registries,
                                            &item_ref,
                                        )
                                        .await
                                        {
                                            Ok((item, _)) => {
                                                if let ArmoryItem::Shuriken { url, .. } = item {
                                                    let resolved_url =
                                                        resolve_shuriken_url(registry_url, &url)?;
                                                    manager.install_url(&resolved_url).await?;
                                                } else {
                                                    eprintln!("{}", format!("Bundle '{}' contains another bundle '{}', nested bundles are not supported", reference.shuriken, shuriken_ref).red());
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("{}", format!("Failed to find shuriken '{}' in registries: {}", shuriken_ref, e).red());
                                            }
                                        }
                                    }
                                    return Ok(()); // After processing all shurikens in the bundle, we can exit
                                }
                            };
                            let resolved_url = resolve_shuriken_url(registry_url, &shuriken_url)?;
                            manager.install_url(&resolved_url).await?;
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                format!("Failed to find shuriken in registries: {}", e).red()
                            );
                            exit(1);
                        }
                    }
                }
            }
        }
        None => {}
    }

    Ok(())
}
