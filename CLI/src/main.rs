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
                .about("List running shuriken services"),
        )
        .get_matches();

    let mut service_manager = ServiceManager::bootstrap()
        .map_err(|e| format!("Failed to initialize service manager: {}", e))?;

    match args.subcommand() {
        Some(("start", shuriken_args)) => {
            let shuriken_name = shuriken_args
                .get_one::<String>("shuriken")
                .expect("Failed to get shuriken name");
            
            match service_manager.start_service(shuriken_name).await {
                Ok(pid) => println!("{}", format!("Started shuriken '{}' with PID {}", shuriken_name, pid).green()),
                Err(e) => eprintln!("{}", format!("Failed to start shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(("stop", shuriken_args)) => {
            let shuriken_name = shuriken_args
                .get_one::<String>("shuriken")
                .expect("Failed to get shuriken name");
            
            match service_manager.stop_service(shuriken_name).await {
                Ok(_) => println!("{}", format!("Stopped shuriken '{}'", shuriken_name).green()),
                Err(e) => eprintln!("{}", format!("Failed to stop shuriken '{}': {}", shuriken_name, e).red()),
            }
        }
        Some(("list", _)) => {
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
        _ => {
            println!("{}", "Invalid action. Use --help for available commands.".red());
        }
    }

    Ok(())
}
