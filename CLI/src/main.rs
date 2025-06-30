// main.rs
use clap::{Arg, Command};
use owo_colors::OwoColorize;

mod service_manager;
use service_manager::ServiceManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Command::new("ninja")
        .version("0.1.0")
        .about("Ninja CLI - Service Manager")
        .subcommand(
            Command::new("start")
                .about("Start a shuriken service")
                .arg(
                    Arg::new("shuriken")
                        .help("The name of the shuriken to start")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop a shuriken service")
                .arg(
                    Arg::new("shuriken")
                        .help("The name of the shuriken to stop")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List running shuriken services")
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("Show all shurikens and their statuses")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .get_matches();

    let mut service_manager = ServiceManager::bootstrap()
        .map_err(|e| format!("Failed to initialize service manager: {}", e))?;

    match args.subcommand() {
        Some(("start", shuriken_args)) => {
            let shuriken_name = shuriken_args
                .get_one::<String>("shuriken")
                .expect("Failed to get shuriken name");

            // Use the actual name from manifest, not service-name
            match service_manager.start_service(shuriken_name).await {
                Ok(pid) => println!("{}", format!("Started shuriken '{}' with PID {}", shuriken_name, pid).green()),
                Err(e) => eprintln!("{}", format!("Failed to start shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(("stop", shuriken_args)) => {
            let shuriken_name = shuriken_args
                .get_one::<String>("shuriken")
                .expect("Failed to get shuriken name");

            // Use the actual name from manifest, not service-name
            match service_manager.stop_service(shuriken_name).await {
                Ok(_) => println!("{}", format!("Stopped shuriken '{}'", shuriken_name).green()),
                Err(e) => eprintln!("{}", format!("Failed to stop shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(("list", list_args)) => {
            let show_all = list_args.get_flag("all");

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
        _ => {
            println!("{}", "Invalid action. Use --help for available commands.".red());
        }
    }

    Ok(())
}