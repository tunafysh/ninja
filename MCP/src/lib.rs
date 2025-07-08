// src/main.rs
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use warp::Filter;

// MCP Protocol Types
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

// MCP-specific types
#[derive(Debug, Serialize, Deserialize)]
struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceCapabilities>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCapabilities {
    #[serde(rename = "listChanged")]
    list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceCapabilities {
    subscribe: Option<bool>,
    #[serde(rename = "listChanged")]
    list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Resource {
    uri: String,
    name: String,
    description: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCallRequest {
    name: String,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(rename = "isError")]
    is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceRequest {
    uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceContent {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    text: Option<String>,
    blob: Option<String>,
}

#[derive(Clone)]
struct SimpleMCP {
    capabilities: ServerCapabilities,
    tools: Vec<Tool>,
    resources: Vec<Resource>,
}

impl SimpleMCP {
    fn new() -> Self {
        SimpleMCP {
            capabilities: ServerCapabilities {
                tools: Some(ToolCapabilities {
                    list_changed: Some(false),
                }),
                resources: Some(ResourceCapabilities {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
            },
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(request.params).await,
            _ => Err(anyhow::anyhow!("Method not found")),
        };

        match result {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self, _params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": self.capabilities,
            "serverInfo": {
                "name": "simple-mcp-server",
                "version": "0.1.0"
            }
        }))
    }

    async fn handle_tools_list(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "tools": self.tools
        }))
    }

    async fn handle_tools_call(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        let tool_call: ToolCallRequest = serde_json::from_value(params)?;

        // Tool implementation goes here
        // For now, return a "not implemented" error
        Ok(serde_json::to_value(ToolResult {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Tool '{}' not implemented", tool_call.name),
            }],
            is_error: Some(true),
        })?)
    }

    async fn handle_resources_list(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "resources": self.resources
        }))
    }

    async fn handle_resources_read(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        let resource_request: ResourceRequest = serde_json::from_value(params)?;

        // Resource implementation goes here
        // For now, return a "not found" error
        Err(anyhow::anyhow!("Resource '{}' not found", resource_request.uri))
    }

    async fn run_stdio(&self) -> Result<()> {
        eprintln!("Simple MCP Server starting (stdio mode)...");
        
        let stdin = tokio::io::stdin();
        let mut reader = AsyncBufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        let mut line = String::new();
        
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                        Ok(request) => {
                            let response = self.handle_request(request).await;
                            let response_json = serde_json::to_string(&response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse JSON-RPC request: {}", e);
                            let error_response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: None,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32700,
                                    message: "Parse error".to_string(),
                                    data: None,
                                }),
                            };
                            let response_json = serde_json::to_string(&error_response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn run_http(&self, port: u16) -> Result<()> {
        eprintln!("Simple MCP Server starting (HTTP mode) on port {}...", port);
        
        let server = self.clone();
        
        // CORS headers
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["POST", "GET", "OPTIONS"]);

        // JSON-RPC endpoint
        let jsonrpc = warp::path("jsonrpc")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |request: JsonRpcRequest| {
                let server = server.clone();
                async move {
                    let response = server.handle_request(request).await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&response))
                }
            });

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&serde_json::json!({
                    "status": "ok",
                    "server": "simple-mcp-server",
                    "version": "0.1.0"
                }))
            });

        // Root endpoint with server info
        let root = warp::path::end()
            .and(warp::get())
            .map(|| {
                warp::reply::json(&serde_json::json!({
                    "name": "Simple MCP Server",
                    "version": "0.1.0",
                    "protocol": "2024-11-05",
                    "endpoints": {
                        "jsonrpc": "/jsonrpc",
                        "health": "/health"
                    }
                }))
            });

        let routes = root
            .or(health)
            .or(jsonrpc)
            .with(cors)
            .with(warp::log("mcp-server"));

        warp::serve(routes)
            .run(([127, 0, 0, 1], port))
            .await;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = SimpleMCP::new();
    
    // Check command line arguments for transport mode
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "--http" {
        // HTTP mode
        let port = if args.len() > 2 {
            args[2].parse().unwrap_or(8080)
        } else {
            8080
        };
        
        server.run_http(port).await
    } else {
        // Default stdio mode
        server.run_stdio().await
    }
}