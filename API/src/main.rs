pub mod config;
pub mod error;
pub mod types;
pub mod manager;
pub mod api;

mod logger;

use clap::{value_parser, Arg, Command};
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup_logger()?;
    info!("Starting Kurokage Service Manager");

    let matches = Command::new("kurokage")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Kurokage - Ninja Engine Service Manager")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_parser(value_parser!(u16))
                .default_value("5671")
                .help("The port to listen on for the REST API")
        )
        .get_matches();

    let port = *matches.get_one::<u16>("port").unwrap_or(&5671);

    // Start the REST API server
    api::run_server(port).await?;

    Ok(())
}