use ninja::{
    dsl::{DslContext, execute_commands},
    manager::ShurikenManager,
};
use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*, // <-- brings in CallToolResult, Content, ServerCapabilities, ServerInfo, Resource, RawResource, etc.
    schemars,
    tool,
    tool_handler,
    tool_router,
};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct ShurikenRequest {
    #[schemars(description = "The name of the shuriken to start/stop.")]
    pub name: String,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListRequest {
    #[schemars(
        description = "If true it also returns the states of the shurikens, else just the names"
    )]
    pub state: bool,
}

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScriptRequest {
    #[schemars(description = "The script to execute.
    If selected the dsl_execute tool, this script will be executed inside ninja's shellscript-like language. 
    if selected the ninjascript tool, this script will be executed using a luau runtime with some custom APIs")]
    pub script: String,
}

#[derive(Clone)]
pub struct Manager {
    manager: ShurikenManager,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
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

    // Small helper for creating text resources (like the SPARQL example)
    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let _ = &self
            .manager
            .start(name.as_str())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Shuriken restarted successfully",
        )]))
    }

    #[tool(description = "List shurikens along with their states if specified.")]
    pub async fn list_shurikens(
        &self,
        Parameters(ListRequest { state }): Parameters<ListRequest>,
    ) -> Result<CallToolResult, McpError> {
        let res = &self
            .manager
            .list(state)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if let Some(left) = res.clone().left() {
            Ok(CallToolResult::success(vec![Content::json(left)?]))
        } else if let Some(right) = res.clone().right() {
            Ok(CallToolResult::success(vec![Content::json(right)?]))
        } else {
            Err(McpError::internal_error(
                "Somehow you managed to turn a boolean ternary. I'm not even mad i'm impressed -- HS.",
                None,
            ))
        }
    }

    #[tool(description = "Execute a script using the ninja dsl")]
    pub async fn dsl_execute(
        &self,
        Parameters(ScriptRequest { script }): Parameters<ScriptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let manager = &self.manager;
        let context = DslContext::new(manager.clone());

        let res = execute_commands(&context, script)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(res.join("\n"))]))
    }

    #[tool(description = "Execute a script using the ninja engine")]
    pub async fn ninjascript_execute(
        &self,
        Parameters(ScriptRequest { script }): Parameters<ScriptRequest>,
    ) -> Result<CallToolResult, McpError> {
        let _ = &self
            .manager
            .engine
            .lock()
            .await
            .execute(&script, Some(&self.manager.root_path), Some(self.manager.clone()))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Script executed successfully",
        )]))
    }

    #[tool(
        description = "Tool for reading the cheatsheet because Resources aren't supported everywhere yet."
    )]
    pub fn read_cheatsheet(&self) -> Result<CallToolResult, McpError> {
        let cheatsheet_path = &self.manager.root_path.join("docs").join("cheatsheet.md");
        let cheatsheet_content = fs::read_to_string(cheatsheet_path).map_err(|e| {
            McpError::internal_error(format!("Failed to read cheatsheet: {}", e), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(
            cheatsheet_content.as_str(),
        )]))
    }

    #[tool(
        description = "Tool for reading the docs because Resources aren't supported everywhere yet."
    )]
    pub fn read_docs(&self) -> Result<CallToolResult, McpError> {
        let home_dir = &self.manager.root_path;
        let cheatsheet_path = home_dir.join("docs").join("cheatsheet.md");
        let cheatsheet_content = fs::read_to_string(cheatsheet_path).map_err(|e| {
            McpError::internal_error(format!("Failed to read cheatsheet: {}", e), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(
            cheatsheet_content.as_str(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for Manager {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                r#"This server provides resources and mostly tools
                for managing shurikens (arbitrary units of other dev software e.g Apache)
                which are: start_shuriken, stop_shuriken, restart_shuriken, shuriken_status and provides tools 
                to execute ninjascript (Luau with a few built-in libraries) and Ninja DSL (a domain-specific language for managing shurikens and interacting with them).
                The cheatsheet for the ninjascript can be found as a resource."#
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }
}
