// src/main.rs
#![allow(dead_code)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use warp::Filter;
use std::collections::HashMap;

// MCP Protocol Types
#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

// MCP-specific types
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts: Option<PromptCapabilities>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCapabilities {
    #[serde(rename = "listChanged")]
    list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResourceCapabilities {
    subscribe: Option<bool>,
    #[serde(rename = "listChanged")]
    list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptCapabilities {
    #[serde(rename = "listChanged")]
    list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Resource {
    uri: String,
    name: String,
    description: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Prompt {
    name: String,
    description: Option<String>,
    arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptArgument {
    name: String,
    description: Option<String>,
    required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolCallRequest {
    name: String,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(rename = "isError")]
    is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResourceRequest {
    uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResourceContent {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    text: Option<String>,
    blob: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptGetRequest {
    name: String,
    arguments: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptMessage {
    role: String,
    content: PromptContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// Tool handler type
type ToolHandler = Box<dyn Fn(&Option<serde_json::Value>) -> ToolResult + Send + Sync>;

// Resource handler type
type ResourceHandler = Box<dyn Fn(&str) -> Result<ResourceContent> + Send + Sync>;

// Prompt handler type
type PromptHandler = Box<dyn Fn(&PromptGetRequest) -> Result<Vec<PromptMessage>> + Send + Sync>;

#[derive(Clone)]
struct SimpleMCP {
    capabilities: ServerCapabilities,
    tools: Vec<Tool>,
    resources: Vec<Resource>,
    prompts: Vec<Prompt>,
    tool_handlers: HashMap<String, ToolHandler>,
    resource_handlers: HashMap<String, ResourceHandler>,
    prompt_handlers: HashMap<String, PromptHandler>,
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
                prompts: Some(PromptCapabilities {
                    list_changed: Some(false),
                }),
            },
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            tool_handlers: HashMap::new(),
            resource_handlers: HashMap::new(),
            prompt_handlers: HashMap::new(),
        }
    }

    // Builder methods for adding tools
    fn add_tool<F>(mut self, tool: Tool, handler: F) -> Self
    where
        F: Fn(&Option<serde_json::Value>) -> ToolResult + Send + Sync + 'static,
    {
        let tool_name = tool.name.clone();
        self.tools.push(tool);
        self.tool_handlers.insert(tool_name, Box::new(handler));
        self
    }

    fn add_resource<F>(mut self, resource: Resource, handler: F) -> Self
    where
        F: Fn(&str) -> Result<ResourceContent> + Send + Sync + 'static,
    {
        let resource_uri = resource.uri.clone();
        self.resources.push(resource);
        self.resource_handlers.insert(resource_uri, Box::new(handler));
        self
    }

    fn add_prompt<F>(mut self, prompt: Prompt, handler: F) -> Self
    where
        F: Fn(&PromptGetRequest) -> Result<Vec<PromptMessage>> + Send + Sync + 'static,
    {
        let prompt_name = prompt.name.clone();
        self.prompts.push(prompt);
        self.prompt_handlers.insert(prompt_name, Box::new(handler));
        self
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(request.params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompts_get(request.params).await,
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

        let result = if let Some(handler) = self.tool_handlers.get(&tool_call.name) {
            handler(&tool_call.arguments)
        } else {
            ToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: format!("Unknown tool: {}", tool_call.name),
                }],
                is_error: Some(true),
            }
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_resources_list(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "resources": self.resources
        }))
    }

    async fn handle_resources_read(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        let resource_request: ResourceRequest = serde_json::from_value(params)?;

        let content = if let Some(handler) = self.resource_handlers.get(&resource_request.uri) {
            handler(&resource_request.uri)?
        } else {
            return Err(anyhow::anyhow!("Resource '{}' not found", resource_request.uri));
        };

        Ok(serde_json::to_value(content)?)
    }

    async fn handle_prompts_list(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "prompts": self.prompts
        }))
    }

    async fn handle_prompts_get(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        let prompt_request: PromptGetRequest = serde_json::from_value(params)?;

        let messages = if let Some(handler) = self.prompt_handlers.get(&prompt_request.name) {
            handler(&prompt_request)?
        } else {
            return Err(anyhow::anyhow!("Prompt '{}' not found", prompt_request.name));
        };

        Ok(serde_json::json!({
            "messages": messages
        }))
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

// Helper functions for creating common tools
fn create_calculator_tool() -> Tool {
    Tool {
        name: "calculator".to_string(),
        description: "Perform basic arithmetic operations".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["operation", "a", "b"]
        }),
    }
}

fn calculator_handler(args: &Option<serde_json::Value>) -> ToolResult {
    let args = match args {
        Some(args) => args,
        None => {
            return ToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: "Missing arguments for calculator".to_string(),
                }],
                is_error: Some(true),
            };
        }
    };

    let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");
    let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let result = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b == 0.0 {
                return ToolResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: "Error: Division by zero".to_string(),
                    }],
                    is_error: Some(true),
                };
            }
            a / b
        }
        _ => {
            return ToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: format!("Unknown operation: {}", operation),
                }],
                is_error: Some(true),
            };
        }
    };

    ToolResult {
        content: vec![ToolContent {
            content_type: "text".to_string(),
            text: format!("{} {} {} = {}", a, operation, b, result),
        }],
        is_error: None,
    }
}

fn create_readme_resource() -> Resource {
    Resource {
        uri: "file://README.md".to_string(),
        name: "Project README".to_string(),
        description: Some("Project documentation and setup instructions".to_string()),
        mime_type: Some("text/markdown".to_string()),
    }
}

fn readme_handler(uri: &str) -> Result<ResourceContent> {
    if uri != "file://README.md" {
        return Err(anyhow::anyhow!("Resource '{}' not found", uri));
    }

    Ok(ResourceContent {
        uri: uri.to_string(),
        mime_type: Some("text/markdown".to_string()),
        text: Some(r#"# Simple MCP Server

A basic Model Context Protocol (MCP) server implementation in Rust.

## Features

- Calculator tool for basic arithmetic operations
- Resource access to project documentation
- Code review prompt generation

## Usage

### Running the server

```bash
cargo run
```

### Available Tools

- `calculator`: Perform basic arithmetic (add, subtract, multiply, divide)

### Available Resources

- `file://README.md`: This project documentation

### Available Prompts

- `code_review`: Generate code review prompts

## Development

Built with Rust using:
- `tokio` for async runtime
- `serde` for JSON serialization
- `warp` for HTTP server
- `anyhow` for error handling

## License

MIT License
"#.to_string()),
        blob: None,
    })
}

fn create_code_review_prompt() -> Prompt {
    Prompt {
        name: "code_review".to_string(),
        description: Some("Generate a code review prompt".to_string()),
        arguments: Some(vec![
            PromptArgument {
                name: "language".to_string(),
                description: Some("Programming language".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "code".to_string(),
                description: Some("Code to review".to_string()),
                required: Some(true),
            },
        ]),
    }
}

fn code_review_handler(request: &PromptGetRequest) -> Result<Vec<PromptMessage>> {
    let language = request.arguments
        .as_ref()
        .and_then(|args| args.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let code = request.arguments
        .as_ref()
        .and_then(|args| args.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok(vec![PromptMessage {
        role: "user".to_string(),
        content: PromptContent {
            content_type: "text".to_string(),
            text: format!(
                "Please review this {} code:\n\n```{}\n{}\n```\n\nProvide feedback on:\n- Code quality and readability\n- Potential bugs or issues\n- Performance considerations\n- Best practices and improvements",
                language, language, code
            ),
        },
    }])
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create server with builder pattern
    let server = SimpleMCP::new()
        .add_tool(create_calculator_tool(), calculator_handler)
        .add_resource(create_readme_resource(), readme_handler)
        .add_prompt(create_code_review_prompt(), code_review_handler);
    
    // Check if HTTP mode is requested
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "http" {
        let port = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8080);
        server.run_http(port).await
    } else {
        server.run_stdio().await
    }
}