use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

pub type McpResult<T> = Result<T, McpError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum TransportMode {
    Stdio,
    Sse { host: String, port: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[async_trait]
pub trait McpHandler: Send + Sync {
    async fn handle_initialize(&self, params: serde_json::Value) -> McpResult<serde_json::Value>;
    async fn handle_list_tools(&self) -> McpResult<Vec<Tool>>;
    async fn handle_call_tool(&self, name: &str, arguments: serde_json::Value) -> McpResult<serde_json::Value>;
    async fn handle_list_resources(&self) -> McpResult<Vec<Resource>>;
    async fn handle_read_resource(&self, uri: &str) -> McpResult<serde_json::Value>;
    async fn handle_list_prompts(&self) -> McpResult<Vec<Prompt>>;
    async fn handle_get_prompt(&self, name: &str, arguments: Option<serde_json::Value>) -> McpResult<serde_json::Value>;
}

pub struct McpServer<H: McpHandler> {
    handler: Arc<H>,
    server_info: ServerInfo,
    transport: TransportMode,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl<H: McpHandler + 'static> McpServer<H> {
    pub fn new(handler: H, server_info: ServerInfo, transport: TransportMode) -> Self {
        Self {
            handler: Arc::new(handler),
            server_info,
            transport,
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) -> McpResult<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        match &self.transport {
            TransportMode::Stdio => {
                self.run_stdio_transport(shutdown_rx).await
            }
            TransportMode::Sse { host, port } => {
                self.run_sse_transport(host.clone(), *port, shutdown_rx).await
            }
        }
    }

    pub async fn stop(&mut self) -> McpResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    async fn run_stdio_transport(&self, mut shutdown_rx: mpsc::Receiver<()>) -> McpResult<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            if let Some(response) = self.process_message(&line).await? {
                                let response_str = serde_json::to_string(&response)?;
                                stdout.write_all(response_str.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                            line.clear();
                        }
                        Err(e) => return Err(McpError::Io(e)),
                    }
                }
            }
        }
        Ok(())
    }

    async fn run_sse_transport(&self, host: String, port: u16, mut shutdown_rx: mpsc::Receiver<()>) -> McpResult<()> {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;
        
        eprintln!("MCP Server listening on http://{}", addr);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((mut stream, _)) => {
                            let handler = Arc::clone(&self.handler);
                            let server_info = self.server_info.clone();
                            
                            tokio::spawn(async move {
                                let mut buffer = Vec::new();
                                let mut temp_buf = [0u8; 1024];
                                
                                // Read HTTP request
                                loop {
                                    match stream.read(&mut temp_buf).await {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            buffer.extend_from_slice(&temp_buf[..n]);
                                            if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                                                break;
                                            }
                                        }
                                        Err(_) => return,
                                    }
                                }

                                let request_str = String::from_utf8_lossy(&buffer);
                                
                                if request_str.contains("GET") && request_str.contains("/sse") {
                                    // Handle SSE connection
                                    let response = "HTTP/1.1 200 OK\r\n\
                                        Content-Type: text/event-stream\r\n\
                                        Cache-Control: no-cache\r\n\
                                        Connection: keep-alive\r\n\
                                        Access-Control-Allow-Origin: *\r\n\r\n";
                                    
                                    if stream.write_all(response.as_bytes()).await.is_err() {
                                        return;
                                    }
                                    
                                    // Keep connection alive for SSE
                                    loop {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                        if stream.write_all(b"data: {\"type\":\"ping\"}\n\n").await.is_err() {
                                            break;
                                        }
                                    }
                                } else if request_str.contains("POST") {
                                    // Handle JSON-RPC POST request
                                    if let Some(body_start) = request_str.find("\r\n\r\n") {
                                        let body = &request_str[body_start + 4..];
                                        
                                        let server = McpServer {
                                            handler,
                                            server_info,
                                            transport: TransportMode::Sse { host: "127.0.0.1".to_string(), port: 8080 },
                                            shutdown_tx: None,
                                        };
                                        
                                        if let Ok(Some(response)) = server.process_message(body).await {
                                            let response_json = serde_json::to_string(&response).unwrap_or_default();
                                            let http_response = format!(
                                                "HTTP/1.1 200 OK\r\n\
                                                Content-Type: application/json\r\n\
                                                Content-Length: {}\r\n\
                                                Access-Control-Allow-Origin: *\r\n\r\n{}",
                                                response_json.len(),
                                                response_json
                                            );
                                            let _ = stream.write_all(http_response.as_bytes()).await;
                                        }
                                    }
                                }
                            });
                        }
                        Err(e) => eprintln!("Failed to accept connection: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_message(&self, message: &str) -> McpResult<Option<JsonRpcResponse>> {
        let message = message.trim();
        if message.is_empty() {
            return Ok(None);
        }

        // Try to parse as JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_str(message)
            .map_err(|e| McpError::Protocol(format!("Invalid JSON-RPC: {}", e)))?;

        if request.jsonrpc != "2.0" {
            return Ok(Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid Request".to_string(),
                    data: None,
                }),
            }));
        }

        let result = match request.method.as_str() {
            "initialize" => {
                let params = request.params.unwrap_or(serde_json::Value::Null);
                match self.handler.handle_initialize(params).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(format!("Initialize failed: {}", e)),
                }
            }
            "tools/list" => {
                match self.handler.handle_list_tools().await {
                    Ok(tools) => Ok(serde_json::json!({ "tools": tools })),
                    Err(e) => Err(format!("List tools failed: {}", e)),
                }
            }
            "tools/call" => {
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let name = params.get("name")
                    .and_then(|v| v.as_str())
                    .expect("Missing tool name");
                let arguments = params.get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                
                match self.handler.handle_call_tool(name, arguments).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(format!("Tool call failed: {}", e)),
                }
            }
            "resources/list" => {
                match self.handler.handle_list_resources().await {
                    Ok(resources) => Ok(serde_json::json!({ "resources": resources })),
                    Err(e) => Err(format!("List resources failed: {}", e)),
                }
            }
            "resources/read" => {
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let uri = params.get("uri")
                    .and_then(|v| v.as_str())
                    .expect("Missing resource URI");
                
                match self.handler.handle_read_resource(uri).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(format!("Read resource failed: {}", e)),
                }
            }
            "prompts/list" => {
                match self.handler.handle_list_prompts().await {
                    Ok(prompts) => Ok(serde_json::json!({ "prompts": prompts })),
                    Err(e) => Err(format!("List prompts failed: {}", e)),
                }
            }
            "prompts/get" => {
                let params = request.params.unwrap_or(serde_json::Value::Null);
                let name = params.get("name")
                    .and_then(|v| v.as_str())
                    .expect("Missing prompt name");
                let arguments = params.get("arguments").cloned();
                
                match self.handler.handle_get_prompt(name, arguments).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(format!("Get prompt failed: {}", e)),
                }
            }
            _ => Err(format!("Unknown method: {}", request.method)),
        };

        let response = match result {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(error_msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: error_msg,
                    data: None,
                }),
            },
        };

        Ok(Some(response))
    }
}

// Example implementation
pub struct ExampleHandler {
    server_info: ServerInfo,
}

impl ExampleHandler {
    pub fn new(server_info: ServerInfo) -> Self {
        Self { server_info }
    }
}

#[async_trait::async_trait]
impl McpHandler for ExampleHandler {
    async fn handle_initialize(&self, _params: serde_json::Value) -> McpResult<serde_json::Value> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": self.server_info
        }))
    }

    async fn handle_list_tools(&self) -> McpResult<Vec<Tool>> {
        Ok(vec![
            Tool {
                name: "echo".to_string(),
                description: "Echo back the input".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to echo back"
                        }
                    },
                    "required": ["message"]
                }),
            }
        ])
    }

    async fn handle_call_tool(&self, name: &str, arguments: serde_json::Value) -> McpResult<serde_json::Value> {
        match name {
            "echo" => {
                let message = arguments.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No message provided");
                
                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Echo: {}", message)
                    }]
                }))
            }
            _ => Err(McpError::InvalidRequest(format!("Unknown tool: {}", name))),
        }
    }

    async fn handle_list_resources(&self) -> McpResult<Vec<Resource>> {
        Ok(vec![])
    }

    async fn handle_read_resource(&self, _uri: &str) -> McpResult<serde_json::Value> {
        Err(McpError::InvalidRequest("No resources available".to_string()))
    }

    async fn handle_list_prompts(&self) -> McpResult<Vec<Prompt>> {
        Ok(vec![])
    }

    async fn handle_get_prompt(&self, _name: &str, _arguments: Option<serde_json::Value>) -> McpResult<serde_json::Value> {
        Err(McpError::InvalidRequest("No prompts available".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_example_handler() {
        let server_info = ServerInfo {
            name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            description: Some("Test server".to_string()),
        };
        
        let handler = ExampleHandler::new(server_info.clone());
        
        // Test initialize
        let init_result = handler.handle_initialize(serde_json::Value::Null).await.unwrap();
        assert!(init_result.get("protocolVersion").is_some());
        
        // Test list tools
        let tools = handler.handle_list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        
        // Test call tool
        let args = serde_json::json!({"message": "Hello, World!"});
        let result = handler.handle_call_tool("echo", args).await.unwrap();
        let content = result.get("content").unwrap().as_array().unwrap();
        assert!(content[0].get("text").unwrap().as_str().unwrap().contains("Hello, World!"));
    }
}