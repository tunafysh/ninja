// main.rs
use std::{path::Path, process::exit};
use ninja::ServiceManager;
use clap::{Parser, Subcommand, Args};
use ::log::info;
use owo_colors::OwoColorize;
use clap_verbosity_flag::Verbosity;

mod log;
use log::setup_logger;


#[derive(Parser)]
#[command(name = "ninja")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Ninja CLI - Service Manager")]
pub struct NinjaCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
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
    List(ListArgs),
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
    
    /// This flag enables REPL mode
    #[arg(long)]
    pub repl: bool,
}

#[derive(Args)]
pub struct ListArgs {
    /// Show all shurikens and their statuses
    #[arg(short = 'f', long)]
    pub full: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    
    let args = NinjaCli::parse();

    setup_logger(args.verbose.into()).expect("Failed to initialize logger");
    
    if args.mcp {
        info!("Starting up in MCP mode.");
        exit(0);
    }

    info!("Initializing service manager...");
    let mut service_manager = ServiceManager::bootstrap()
        .map_err(|e| format!("Failed to initialize service manager: {}", e))?;

    match args.command {
        Some(Commands::Start(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;
        
            info!("Starting shuriken {}...", shuriken_name);
            // Use the actual name from manifest, not service-name
            match service_manager.start_service(shuriken_name.as_str()).await {
                Ok(pid) => println!("{}", format!("Started shuriken '{}' with PID {}", shuriken_name, pid).green()),
                Err(e) => eprintln!("{}", format!("Failed to start shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(Commands::Stop(shuriken_args)) => {
            let shuriken_name = shuriken_args.shuriken;

            info!("Stopping shuriken {}...", shuriken_name);
            // Use the actual name from manifest, not service-name
            match service_manager.stop_service(shuriken_name.as_str()).await {
                Ok(_) => println!("{}", format!("Stopped shuriken '{}'", shuriken_name).green()),
                Err(e) => eprintln!("{}", format!("Failed to stop shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(Commands::List(list_args)) => {
            let show_all = list_args.full;

            if show_all {
                // Show all services with their statuses
                match service_manager.get_all_services().await {
                    Ok(services) => {
                        if services.is_empty() {
                            // If no services in database, show available services from configs
                            let available_services = service_manager.list_services();
                            if available_services.is_empty() {
                                println!("{}", "No shurikens found".yellow());
                            } else {
                                println!("{}", "Available shurikens:".bold());
                                for service_name in available_services {
                                    println!("  • {} {}", service_name.blue(), "stopped".dimmed());
                                }
                            }
                        } else {
                            // Get all available services from configs
                            let available_services = service_manager.list_services();
                            let mut all_services = std::collections::HashMap::new();
                            
                            // Initialize all services as stopped
                            for service_name in available_services {
                                all_services.insert(service_name, ("stopped".to_string(), None));
                            }
                            
                            // Update with actual statuses from database
                            for service in services {
                                all_services.insert(service.name, (service.status, service.pid));
                            }
                            
                            println!("{}", "All shurikens:".bold());
                            for (name, (status, pid)) in all_services {
                                let status_colored = match status.as_str() {
                                    "running" => status.green().to_string(),
                                    "stopped" => status.red().to_string(),
                                    _ => status.yellow().to_string(),
                                };
                                
                                let pid_str = pid
                                    .map(|p| format!(" (PID: {})", p))
                                    .unwrap_or_default();
                                    
                                println!("  • {} {} {}", name.blue(), status_colored, pid_str.dimmed());
                            }
                        }
                    }
                    Err(e) => eprintln!("{}", format!("Failed to get services: {}", e).red()),
                }
            } else {
                // Show only running services (original behavior)
                match service_manager.get_running_services().await {
                    Ok(services) => {
                        if services.is_empty() {
                            println!("{}", "No running services".yellow());
                        } else {
                            println!("{}", "Running services:".bold());
                            for service in services {
                                let pid_str = service.pid
                                    .map(|p| format!(" (PID: {})", p))
                                    .unwrap_or_default();
                                println!("  • {}{}", service.name.green(), pid_str.dimmed());
                            }
                        }
                    }
                    Err(e) => eprintln!("{}", format!("Failed to get running services: {}", e).red()),
                }
            }
        }
        Some(Commands::Run(script_args)) => {
            let file_arg = script_args.file_script.expect("Failed to get file path");
            let content = file_arg.as_str();
            let rt = ninja_engine::NinjaEngine::new();
            
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
        _ => {
            println!("{}", "Invalid action. Use --help for available commands.".red());
        }
    }

    Ok(())
}
