use serde::{Deserialize, Serialize};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::json;

#[derive(Debug, Deserialize)]
struct McpRequest {
    id: String,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    id: String,
    result: serde_json::Value,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = io::stdout();

    // 1. On startup, announce capabilities
    let capabilities = json!({
        "tools": [
            { "name": "ping", "description": "Check if server is alive" },
            { "name": "echo", "description": "Echo back input" }
        ],
        "resources": [
            { "name": "config", "description": "Access Ninja config file" }
        ],
        "prompts": [
            { "name": "hello", "description": "Generate a greeting" }
        ]
    });

    let announce = json!({
        "jsonrpc": "2.0",
        "method": "mcp/announce",
        "params": capabilities
    });

    stdout.write_all(announce.to_string().as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;

    // 2. Event loop: handle requests
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<McpRequest>(&line) {
            Ok(req) => {
                let result = handle_request(&req.method, req.params).await;
                let response = McpResponse { id: req.id, result };

                let json = serde_json::to_string(&response).unwrap();
                stdout.write_all(json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
            Err(e) => {
                eprintln!("Failed to parse request: {e}");
            }
        }
    }

    Ok(())
}

/// Implement MCP tools/resources/prompts here
async fn handle_request(method: &str, params: serde_json::Value) -> serde_json::Value {
    match method {
        "ping" => json!({ "msg": "pong" }),
        "echo" => json!({ "echo": params }),
        "get_config" => {
            // Example: fetch Ninja config path
            json!({ "config_path": "shurikens/test.shuriken" })
        }
        "hello" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("world");
            json!({ "greeting": format!("Hello, {name}!") })
        }
        _ => json!({ "error": format!("Unknown method: {method}") }),
    }
}
