pub mod config;
pub mod error;
pub mod types;
pub mod manager;

mod logger;

use log::info; 
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup_logger()?;
    info!("Starting Kurokage");
    // let _manager = manager::ServiceManager::bootstrap()?;

    let list = warp::path!("shurikens" / String)
        .and(warp::get())
        .map(|name| format!("Hello, {}!", name));

    

    warp::serve(list)
        .run(([127, 0, 0, 1], 5671))
        .await;

    info!("Kurokage is running on port 5671");

    Ok(())
    
}