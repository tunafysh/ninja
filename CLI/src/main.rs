use ::log::info;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use ninja::{
    VERSION,
    common::{
        config::{
            NinjaConfig, ShurikenReference, fetch_registries, find_shuriken_in_registries,
            get_shuriken_info, resolve_shuriken_url,
        },
        registry::ArmoryItem,
        types::{ArmoryMetadata, FieldValue, PlatformPath, ShurikenState},
    },
    manager::ShurikenManager,
    shuriken::{ManagementType, Shuriken, ShurikenConfig, ShurikenMetadata},
};
use ninja_api::server;
use ninja_mcp::server as mcpserver;
use owo_colors::OwoColorize;
use std::{
    collections::HashMap,
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
            let theme = ColorfulTheme::default();
            let management_types = ["native", "script"];

            println!("{}", "Manifest section".bold().blue());

            let name: String = Input::with_theme(&theme)
                .with_prompt("Enter the name of the shuriken")
                .interact_text()?;

            let id: String = Input::with_theme(&theme)
                .with_prompt("Enter the service name")
                .interact_text()?;

            let version: String = Input::with_theme(&theme)
                            .with_prompt("Enter the version of the shuriken (this is required if you're planning to upload to Armory)")
                            .allow_empty(true)
                            .interact_text()?;

            // ===== Maintenance prompt =====
            let management_choice = Select::with_theme(&theme)
                .with_prompt("Enter the management type (native/script)")
                .items(&management_types)
                .default(0)
                .interact()?;

            let management = match management_types[management_choice] {
                "native" => {
                    let bin_path_windows: String = Input::with_theme(&theme)
                        .with_prompt("Enter the binary path for Windows systems")
                        .interact_text()?;

                    let bin_path_unix: String = Input::with_theme(&theme)
                        .with_prompt("Enter the binary path for Unix systems")
                        .interact_text()?;

                    let args = {
                        let input: String = Input::with_theme(&theme)
                            .with_prompt("Enter arguments (optional, comma-separated)")
                            .allow_empty(true)
                            .interact_text()?;
                        (!input.trim().is_empty())
                            .then(|| input.split(',').map(|s| s.trim().to_string()).collect())
                    };

                    ManagementType::Native {
                        bin_path: PlatformPath::Platform {
                            windows: bin_path_windows,
                            unix: bin_path_unix,
                        },
                        args,
                        cwd: None,
                    }
                }
                "script" => {
                    let script_path: String = Input::with_theme(&theme)
                        .with_prompt("Enter the script path")
                        .interact_text()?;

                    let script_path = PathBuf::from(script_path);

                    ManagementType::Script { script_path }
                }
                _ => {
                    eprintln!("Invalid management type selected.");
                    exit(1);
                }
            };

            // ===== Shuriken type prompt (tagged struct) =====
            let shuriken_options = ["daemon", "executable"];
            let choice = Select::with_theme(&theme)
                .with_prompt("Enter the shuriken type")
                .items(&shuriken_options)
                .default(0)
                .interact()?;

            let admin = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("Require administrator priviliges to launch?")
                .default(false)
                .interact()?;

            // ===== Config section =====

            println!("{}", "Config section".bold().blue());
            let config_enabled = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("Add config?")
                .default(false)
                .interact()?;

            let (conf_path, options) = if config_enabled {
                let conf_input: String = Input::with_theme(&theme)
                    .with_prompt("Enter config path for the templater to output (e.g. for Apache 'conf/httpd.conf')")
                    .interact_text()?;

                let conf_path = PathBuf::from(conf_input);

                // Ask whether to add configuration options interactively
                let add_options = dialoguer::Confirm::with_theme(&theme)
                    .with_prompt("Add configuration options?")
                    .default(false)
                    .interact()?;

                let mut options_map: Option<HashMap<String, FieldValue>> = None;

                if add_options {
                    let mut map = HashMap::new();

                    loop {
                        let key: String = Input::with_theme(&theme)
                            .with_prompt("Enter option key (leave empty to finish)")
                            .allow_empty(true)
                            .interact_text()?;

                        if key.trim().is_empty() {
                            break;
                        }

                        let value: String = Input::with_theme(&theme)
                            .with_prompt("Enter value for this key")
                            .interact_text()?;

                        let value: FieldValue = FieldValue::from(value);

                        map.insert(key, value);
                    }

                    options_map = Some(map);
                }

                (Some(conf_path), options_map)
            } else {
                (None, None)
            };

            println!("{}", format!("Generating manifest for '{}'", name).bold());

            let manifest = Shuriken {
                metadata: ShurikenMetadata {
                    name: name.clone(),
                    id: id.clone(),
                    version,
                    management: Some(management),
                    shuriken_type: shuriken_options[choice].to_string(),
                    require_admin: admin,
                },
                config: conf_path.map(|path| ShurikenConfig {
                    config_path: path,
                    options: None,
                }),
                logs: None,
                tools: None,
            };

            create_dir_all(format!("shurikens/{}/.ninja", name)).unwrap_or_else(|_| {
                eprintln!("Failed to create directory for shuriken '{}'", name);
                exit(1);
            });

            env::set_current_dir(format!("shurikens/{}/.ninja", name))?;

            if let Some(opts) = options {
                let serialized_options = toml::ser::to_string_pretty(&opts)?;
                fs::write("config.tmpl", "").await?;
                fs::write("options.toml", serialized_options).await?;
            }

            let manifest_path = PathBuf::from("manifest.toml");
            let mut file = File::create(&manifest_path).unwrap_or_else(|_| {
                eprintln!("Failed to create manifest file for shuriken '{}'", name);
                exit(1);
            });

            if let Some(management) = &manifest.metadata.management
                && let ManagementType::Script { script_path } = management
            {
                if let Some(parent) = script_path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::write(
                        script_path,
                        "function start()\n\t-- Start procedure goes here\nend\n\nfunction stop()\n\t-- Stop procedure goes here\nend",
                    )
                    .await?;
            }

            let toml_content = toml::to_string(&manifest).unwrap_or_else(|_| {
                eprintln!("Failed to serialize manifest for shuriken '{}'", name);
                exit(1);
            });

            file.write_all(toml_content.as_bytes()).unwrap_or_else(|_| {
                eprintln!("Failed to write manifest file for shuriken '{}'", name);
                exit(1);
            });

            env::set_current_dir(&manager.root_path)?;

            println!("Manifest for '{}' generated successfully!", name);
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
            use dialoguer::{Input, theme::ColorfulTheme};
            use serde_json::from_str;
            use tokio::fs;

            if let Some(config_path) = args.options {
                // --- Load metadata from config file ---
                let serialized_metadata = fs::read_to_string(&config_path).await?;
                let metadata: ArmoryMetadata = from_str(&serialized_metadata)?;

                println!("{}", "Creating shuriken...".bold());

                // No need to manually create "blacksmith" here,
                // `forge` already ensures the directory exists.
                manager.forge(metadata, args.path).await?;
            } else {
                let theme = ColorfulTheme::default();

                let name: String = Input::with_theme(&theme)
                    .with_prompt("Enter the name of the shuriken")
                    .interact_text()?;

                let id: String = Input::with_theme(&theme)
                    .with_prompt("Enter the id for this shuriken (Apache -> httpd)")
                    .interact_text()?;

                let platform: String = Input::with_theme(&theme)
                    .with_prompt(
                        "Enter the platform this shuriken was designed for \
                             (target triple is preferred but something like \
                             windows-x86_64 is allowed)",
                    )
                    .interact_text()?;

                let version: String = Input::with_theme(&theme)
                    .with_prompt(
                        "Enter the version for this shuriken \
                             (semver is preferred but anything with numbers will suffice)",
                    )
                    .interact_text()?;

                let postinstall: Option<PathBuf> = Input::<String>::with_theme(&theme)
                        .with_prompt(
                            "Path for postinstall script (starts from the path you provided as argument, optional)",
                        )
                        .allow_empty(true)
                        .interact_text()
                        .ok()
                        .and_then(|s| {
                            let s = s.trim();
                            if s.is_empty() {
                                None
                            } else {
                                Some(PathBuf::from(s))
                            }
                        });

                let description: Option<String> = Input::<String>::with_theme(&theme)
                        .with_prompt(
                            "Description for the shuriken (will be displayed on the install menu, optional)",
                        )
                        .allow_empty(true)
                        .interact_text()
                        .ok()
                        .and_then(|s| {
                            let s = s.trim().to_string();
                            if s.is_empty() { None } else { Some(s) }
                        });

                let synopsis: Option<String> = Input::<String>::with_theme(&theme)
                        .with_prompt(
                            "Synopsis (short description) for the shuriken (will be displayed on the install menu, optional)",
                        )
                        .allow_empty(true)
                        .interact_text()
                        .ok()
                        .and_then(|s| {
                            let s = s.trim().to_string();
                            if s.is_empty() { None } else { Some(s) }
                        });

                let authors: Option<Vec<String>> = Input::<String>::with_theme(&theme)
                    .with_prompt("Authors of this shuriken (optional, comma-separated)")
                    .allow_empty(true)
                    .interact_text()
                    .ok()
                    .map(|s| {
                        s.split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect::<Vec<_>>()
                    })
                    .and_then(|v| if v.is_empty() { None } else { Some(v) });

                let repository: Option<String> = Input::<String>::with_theme(&theme)
                    .with_prompt("The repository URL for this shuriken (optional)")
                    .allow_empty(true)
                    .interact_text()
                    .ok()
                    .and_then(|s| {
                        let s = s.trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    });

                let license: Option<String> = Input::<String>::with_theme(&theme)
                    .with_prompt(
                        "The license or licenses the software in this shuriken use \
                             (GPL, MIT or anything similar, optional)",
                    )
                    .allow_empty(true)
                    .interact_text()
                    .ok()
                    .and_then(|s| {
                        let s = s.trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    });

                println!("{}", format!("Generating metadata for '{}'", &name).bold());

                let metadata = ArmoryMetadata {
                    name,
                    id,
                    platform,
                    version,
                    postinstall,
                    description,
                    authors,
                    license,
                    synopsis,
                    repository,
                };

                println!("{}", "Creating shuriken...".bold());
                manager.forge(metadata, args.path).await?;
            }
        }
        Some(Commands::Remove(args)) => {
            manager.remove(&args.shuriken).await?;
        }
        Some(Commands::Registry(registry_args)) => {
            match registry_args.subcommand {
                RegistrySubcommands::Get(get_args) => {
                    info!(
                        "Fetching info for shuriken reference: {}",
                        get_args.reference
                    );

                    let reference = ShurikenReference::parse(&get_args.reference)?;
                    let config = NinjaConfig::new();
                    let registries = fetch_registries(&config).await;

                    match get_shuriken_info(&registries, &reference) {
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
                    let config = NinjaConfig::new();
                    let registries = fetch_registries(&config).await;
                    match find_shuriken_in_registries(&registries, &reference) {
                        Ok((shuriken, registry_name)) => {
                            let registry_url =
                                config.get_registries().get(&registry_name).ok_or_else(|| {
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
                                        match find_shuriken_in_registries(&registries, &item_ref) {
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
