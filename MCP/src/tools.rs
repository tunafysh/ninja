use ninja::manager::ShurikenManager;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct ShurikenRequest {
    #[schemars(description = "The name of the shuriken to start/stop.")]
    pub name: String,
}

pub struct Manager {
    manager: ShurikenManager,
    tool_router: ToolRouter<Self>,
}

#[tool_router( router = tool_router)]
impl Manager {
    pub async fn new() -> Self {
        let manager = ShurikenManager::new()
            .await
            .map_err(|e| {
                println!("{}", e);
            })
            .unwrap();

        Self {
            manager,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Start the corresponding shuriken")]
    pub async fn start_shuriken(
        &self,
        Parameters(ShurikenRequest { name }): Parameters<ShurikenRequest>,
    ) -> Result<CallToolResult, McpError> {
        let _ = &self
            .manager
            .start(name.as_str())
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Shuriken started successfully",
        )]))
    }

    #[tool(description = "Stop the corresponding shuriken")]
    pub async fn stop_shuriken(
        &self,
        Parameters(ShurikenRequest { name }): Parameters<ShurikenRequest>,
    ) -> Result<CallToolResult, McpError> {
        let _ = &self
            .manager
            .stop(name.as_str())
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Shuriken stopped successfully",
        )]))
    }

    #[tool(description = "Restart the corresponding shuriken")]
    pub async fn restart_shuriken(
        &self,
        Parameters(ShurikenRequest { name }): Parameters<ShurikenRequest>,
    ) -> Result<CallToolResult, McpError> {
        let _ = &self
            .manager
            .stop(name.as_str())
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let _ = &self
            .manager
            .start(name.as_str())
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Shuriken restarted successfully",
        )]))
    }

    #[tool(description = "Get the status of the corresponding shuriken")]
    pub async fn shuriken_status(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "Shuriken started successfully",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for Manager {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: None,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
