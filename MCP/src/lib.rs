use log::info;
use rmcp::{ErrorData, ServiceExt};
use tokio::io::{stdin, stdout};

mod tools;

pub async fn server() -> Result<(), ErrorData> {
    let transport = (stdin(), stdout());

    let service = tools::Manager::new();

    info!("Manager made");

    let server = service
        .await
        .serve(transport)
        .await
        .map_err(|e| ErrorData::new(rmcp::model::ErrorCode(-1), e.to_string(), None))?;

    info!("Server made, Running.");
    
    server
        .waiting()
        .await
        .map_err(|e| ErrorData::new(rmcp::model::ErrorCode(-2), e.to_string(), None))?;
    Ok(())
}
