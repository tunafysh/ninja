// main.rs
use std::{fs::{create_dir_all, File}, io::Write, path::{Path, PathBuf}, process::exit};
use ninja::{ api::server, config::{MaintenanceType, ShurikenConfig}, manager::ShurikenManager, shuriken::Shuriken, types::{PlatformPath, ShurikenState}};
use clap::{Parser, Subcommand, Args};
use ::log::info;
use owo_colors::OwoColorize;
use clap_verbosity_flag::Verbosity;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use ninja_mcp::server as mcpserver;

mod log;
use log::setup_logger;

mod repl;
use repl::repl_mode;

#[derive(Parser)]
#[command(name = "ninja")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Ninja CLI - The command line version of ninja")]
pub struct NinjaCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    #[arg(long)]
    pub repl: bool,

    #[arg(long, hide = true)]
    pub mcp: bool,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a shuriken
    Start(StartArgs),
    /// Stop a shuriken
    Stop(StopArgs),
    /// Run a script using the Ninja Runtime
    Run(RunArgs),
    /// List running shuriken services
    List,
    /// Generate a manifest file
    Manifest,
    /// Start shurikenctl as an API endpoint (hidden)
    Api(ApiArgs),
    /// Configure a shuriken using the config DSL specified in docs
    Config(ConfigArgs)
}

#[derive(Args)]
pub struct ConfigArgs {
    #[arg(help = "the name of the shuriken to configure.")]
    pub shuriken: String,
    #[arg(last = true)]
    pub command: String,
}

#[derive(Args)]
#[clap(hide = true)]
pub struct ApiArgs {
    pub port: u16
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
pub struct RunArgs {
    /// The path of the file or snippet of script to run
    #[arg(name = "file/script")]
    pub file_script: Option<String>,
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
        return Ok(())
    }

    info!("Initializing service manager...");
    let manager = ShurikenManager::new().await
        .map_err(|e| format!("Failed to initialize service manager: {}", e))?;

    match args.command {
        Some(Commands::Start(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;
        
            println!("Starting shuriken {}...\n", shuriken_name);
            // Use the actual name from manifest, not service-name
            match manager.start(shuriken_name.as_str()).await {
                Ok(_) => println!("{}", format!("\nStarted shuriken '{}'", shuriken_name.green())),
                Err(e) => eprintln!("{}", format!("Failed to start shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(Commands::Stop(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;
            
            println!("Stopping shuriken {}...\n", shuriken_name);
            // Use the actual name from manifest, not service-name
            match manager.stop(shuriken_name.as_str()).await {
                Ok(_) => println!("{}", format!("\nStopped shuriken '{}'", shuriken_name.red())),
                Err(e) => eprintln!("{}", format!("Failed to stop shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(Commands::List) => {
            
            let partial_shurikens = manager.list(true).await?.left(); 
            if partial_shurikens.is_some() {
                let shurikens = partial_shurikens.unwrap();
                

                println!("{}", "Shurikens:\n".blue().bold());
                for (name, state) in shurikens {
                    if state == ShurikenState::Running {
                        println!("{} {}", name, "running".green());
                    }
                    else {
                        println!("{} {}", name, "stopped".red());
                    }
                }
            }
            else {
                eprintln!("Failed to list shurikens, None returned.")
            }
            
            println!(); // for styling purposes
        }
        Some(Commands::Run(script_args)) => {
            let file_arg = script_args.file_script.ok_or("path argument is empty")?;
            let content = file_arg.as_str();
            let rt = ninja_engine::NinjaEngine::new().map_err(|e| {
                eprintln!("Failed to initialize Ninja engine: {}", e);
                exit(1);
            })?;

            if Path::new(content).exists() {
                match rt.execute_file(content) {
                    Ok(_) => exit(0),
                    Err(e) => eprintln!("Error: {}", e),
                }
            } else {
                match rt.execute(content) {
                    Ok(_) => exit(0),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Some(Commands::Manifest) => {
            let theme = ColorfulTheme::default();
            let maintenance_types = ["native", "script"];

            let name: String = Input::with_theme(&theme)
                .with_prompt("Enter the name of the shuriken")
                .interact_text()
                .unwrap();

            let service_name: String = Input::with_theme(&theme)
                .with_prompt("Enter the service name")
                .interact_text()
                .unwrap();

            // ===== Maintenance prompt =====
            let maintenance_choice = Select::with_theme(&theme)
                .with_prompt("Enter the maintenance type (native/script)")
                .items(&maintenance_types)
                .default(0)
                .interact()
                .unwrap();

            let maintenance = match maintenance_types[maintenance_choice] {
                "native" => {
                    let bin_path_windows: String = Input::with_theme(&theme)
                        .with_prompt("Enter the binary path for Windows systems")
                        .interact_text()
                        .unwrap();
                
                    let bin_path_unix: String = Input::with_theme(&theme)
                        .with_prompt("Enter the binary path for Unix systems")
                        .interact_text()
                        .unwrap();
                
                    let config_path = {
                        let input: String = Input::with_theme(&theme)
                            .with_prompt("Enter the config path (optional)")
                            .allow_empty(true)
                            .interact_text()
                            .unwrap();
                        (!input.trim().is_empty()).then_some(input)
                    };
                
                    let args = {
                        let input: String = Input::with_theme(&theme)
                            .with_prompt("Enter arguments (optional, comma-separated)")
                            .allow_empty(true)
                            .interact_text()
                            .unwrap();
                        (!input.trim().is_empty())
                            .then(|| input.split(',').map(|s| s.trim().to_string()).collect())
                    };
                
                    MaintenanceType::Native {
                        bin_path: PlatformPath::Platform {
                            windows: bin_path_windows,
                            unix: bin_path_unix,
                        },
                        config_path: config_path.map(PathBuf::from),
                        args,
                    }
                }
                "script" => {
                    let script_path: String = Input::with_theme(&theme)
                        .with_prompt("Enter the script path")
                        .interact_text()
                        .unwrap();
                    MaintenanceType::Script {
                        script_path: PathBuf::from(script_path),
                    }
                }
                _ => {
                    eprintln!("Invalid maintenance type selected.");
                    exit(1);
                }
            };
        
            // ===== Shuriken type prompt (tagged struct) =====
                let options = ["daemon", "executable"];
                let choice = Select::with_theme(&theme)
                    .with_prompt("Enter the shuriken type")
                    .items(&options)
                    .default(0)
                    .interact()
                    .unwrap();

            let shuriken_type = match options[choice] {
                "daemon" => "Daemon",
                "executable" => "Executable",
                _ => ""
            };
            
            
            let add_path = dialoguer::Confirm::with_theme(&theme)
                            .with_prompt("Add to PATH?")
                            .default(false)
                            .interact()
                            .unwrap();
        
            println!("Generating manifest for '{}'", name);
            let manifest = Shuriken {
                shuriken: ShurikenConfig {
                    name: name.clone(),
                    service_name: service_name.clone(),
                    maintenance,
                    shuriken_type: shuriken_type.to_string(),
                    add_path
                },
                config: None,
                logs: None,
            };
        
            create_dir_all(format!("shurikens/{}", name)).unwrap_or_else(|_| {
                eprintln!("Failed to create directory for shuriken '{}'", name);
                exit(1);
            });
        
            let manifest_path = PathBuf::from(format!("shurikens/{}/manifest.toml", name));
            let mut file = File::create(&manifest_path).unwrap_or_else(|_| {
                eprintln!("Failed to create manifest file for shuriken '{}'", name);
                exit(1);
            });
        
            let toml_content = toml::to_string(&manifest).unwrap_or_else(|_| {
                eprintln!("Failed to serialize manifest for shuriken '{}'", name);
                exit(1);
            });
        
            file.write_all(toml_content.as_bytes()).unwrap_or_else(|_| {
                eprintln!("Failed to write manifest file for shuriken '{}'", name);
                exit(1);
            });
        
            println!("Manifest for '{}' generated successfully!", name);
        },
        Some(Commands::Api(args)) => {
            info!("Starting API endpoint with port {}", args.port);
            server(args.port).await?;
        },
        Some(Commands::Config(args)) => {
            info!("Configuring shuriken {} with parameters {}", args.shuriken, args.command)
        },
        None => eprintln!("No subcommand selected. Exiting...")
    }

    Ok(())
}
