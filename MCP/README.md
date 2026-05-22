# Ninja MCP: Model Context Protocol Support

Integrate Ninja with AI tools and language models through the Model Context Protocol. Expose Ninja's service management capabilities to Claude, GPT, and other AI agents for intelligent automation and orchestration.

## Overview

The Ninja MCP server allows AI agents and language models to:
* Query service status and configuration
* Start, stop, and restart services
* Install and manage packages
* Execute automation scripts
* Perform backups and restore operations
* Access logs and diagnostics

This enables natural-language control of infrastructure and empowers AI assistants to reason about and manage your services.

## Features

- **Full Ninja Integration**: Access all core Ninja functionality
- **MCP Protocol Compliance**: Implements the Model Context Protocol specification
- **Tool-Based Interface**: Structured tools for reliable AI interaction
- **Type Safety**: JSON schema validation for all operations
- **Error Handling**: Clear error messages for debugging
- **Logging Support**: Integrated with Ninja's logging system
- **Resource Access**: Fetch documentation and cheatsheets for context

## Getting Started

### Installation

Build the MCP server as part of the workspace:

```bash
cargo build --release
```

The compiled binary will be in `target/release/ninja-mcp` (or `ninja-mcp.exe` on Windows).

### Configuration

Register the MCP server with your AI tool. Configuration depends on your client, but typically looks like:

```json
{
  "mcpServers": {
    "ninja": {
      "command": "/path/to/ninja-mcp",
      "args": ["--port", "3001"]
    }
  }
}
```

### Usage with Claude/ChatGPT

Once configured, you can use natural language to control services:

```
"Start the web server and check its status"
"Create a backup of the database service"
"List all services and tell me which ones are running"
"Install a new package from ~/packages/service.shuriken"
```

The AI agent will:
1. Parse your intent
2. Map it to appropriate Ninja tools
3. Execute the operations
4. Interpret results and provide feedback

## Available Tools

The MCP server exposes the following tools:

### Service Management

#### list_shurikens
List all installed shurikens and their states.

Parameters: None

Returns: Array of shuriken information including name, version, state, and type.

#### get_shuriken_status
Get detailed status information for a specific shuriken.

Parameters:
- `name` (string, required): The shuriken name

Returns: Status details including running state, uptime, PID, and resource usage.

#### start_shuriken
Start a shuriken service.

Parameters:
- `name` (string, required): The shuriken name

Returns: Operation result with success status and message.

#### stop_shuriken
Stop a running shuriken service.

Parameters:
- `name` (string, required): The shuriken name

Returns: Operation result with success status and message.

#### restart_shuriken
Restart a shuriken service.

Parameters:
- `name` (string, required): The shuriken name

Returns: Operation result with success status and message.

### Package Management

#### install_shuriken
Install a new shuriken from a file path.

Parameters:
- `path` (string, required): Path to the .shuriken file

Returns: Operation result with installation details.

#### remove_shuriken
Remove a shuriken completely.

Parameters:
- `name` (string, required): The shuriken name

Returns: Operation result confirming removal.

### Configuration

#### configure_shuriken
Generate configuration from template and save it.

Parameters:
- `name` (string, required): The shuriken name
- `options` (object, optional): Configuration overrides

Returns: Generated configuration content.

#### get_configuration
Retrieve the current configuration for a shuriken.

Parameters:
- `name` (string, required): The shuriken name

Returns: Configuration as JSON or TOML.

### Backup & Restore

#### create_backup
Create a backup of a shuriken's state and configuration.

Parameters:
- `name` (string, required): The shuriken name
- `label` (string, optional): Backup label for identification

Returns: Backup information including timestamp and location.

#### restore_backup
Restore a shuriken from a backup.

Parameters:
- `name` (string, required): The shuriken name
- `backup_id` (string, required): Backup identifier

Returns: Operation result confirming restoration.

#### list_backups
List available backups for a shuriken.

Parameters:
- `name` (string, required): The shuriken name

Returns: Array of available backups with metadata.

### Utilities

#### run_script
Execute a Ninja script (Lua or DSL).

Parameters:
- `script_path` (string, required): Path to script file
- `args` (array, optional): Arguments to pass to script

Returns: Script output and exit status.

#### get_logs
Retrieve recent logs from a shuriken.

Parameters:
- `name` (string, required): The shuriken name
- `lines` (integer, optional): Number of lines to retrieve (default: 50)

Returns: Log entries with timestamps.

## Resource Access

The MCP server also provides access to documentation resources:

### get_documentation
Access the full Ninja documentation.

Returns: Markdown documentation for reference.

### get_cheatsheet
Get the Ninja command cheatsheet.

Returns: Quick reference guide for common operations.

## Example Interactions

### Natural Language Examples

```
"Can you list all running services and restart the web server?"

Response: Lists all services and executes restart.

"Back up my database before updating its configuration"

Response: Creates backup, confirms success.

"Monitor the cache service and tell me if it goes down"

Response: Queries status repeatedly and alerts on state change.

"Show me the logs from the last hour for the API server"

Response: Retrieves and displays relevant log entries.
```

### Programmatic Usage

If using the Ninja MCP SDK directly:

```rust
use ninja_mcp::Client;

#[tokio::main]
async fn main() {
    let client = Client::connect("http://localhost:3001").await.unwrap();

    // List shurikens
    let shurikens = client.list_shurikens().await.unwrap();
    println!("{:?}", shurikens);

    // Start a service
    client.start_shuriken("webserver").await.unwrap();

    // Get status
    let status = client.get_shuriken_status("webserver").await.unwrap();
    println!("{:?}", status);
}
```

## Architecture

The MCP server is structured as:

1. **Protocol Handler**: Implements MCP protocol specification
2. **Tool Registry**: Defines available tools and their schemas
3. **Ninja Integration**: Bridges MCP calls to core Ninja functions
4. **Protocol Conversion**: Translates between MCP format and Ninja types
5. **Error Handling**: Gracefully handles errors and returns meaningful messages

## Development

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
cargo test
```

### Logging

Enable detailed logging:

```bash
RUST_LOG=debug ninja-mcp --port 3001
```

## Integration Patterns

### With Claude

```
User: "Use Ninja MCP to check service status and restart if needed"

Claude: I'll use the Ninja MCP tools to check the service status first.
[Calls list_shurikens and get_shuriken_status]

The webserver is currently idle. I'll start it now.
[Calls start_shuriken]

The webserver has been successfully started.
```

### With ChatGPT

Configure in OpenAI's system prompt or custom instructions to delegate infrastructure tasks to the Ninja MCP server.

### With Other Agents

Any AI system supporting the Model Context Protocol can use Ninja MCP for autonomous service management.

## Security Considerations

When exposing Ninja through MCP:

1. **Network Security**: Run MCP server on trusted networks only
2. **Access Control**: Restrict who can connect to the MCP server
3. **Audit Logging**: All MCP operations are logged for accountability
4. **API Limits**: Consider rate limiting for production deployments
5. **Capability Restrictions**: Disable dangerous operations if needed

Example security wrapper:

```bash
# Run MCP server on localhost only, behind firewall
ninja-mcp --bind 127.0.0.1 --port 3001
```

## Troubleshooting

### Connection Refused

Verify the MCP server is running:

```bash
ps aux | grep ninja-mcp
```

### Tool Not Found

Ensure the specific Ninja tool is available:

```bash
shurikenctl list  # Verify Ninja itself works
```

### Slow Operations

Check system resources and Ninja logs:

```bash
RUST_LOG=debug ninja-mcp --port 3001
```

## Documentation

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [Ninja Core Documentation](../core/README.md)
- [CLI Reference](https://ninja-rs.vercel.app/docs/reference/cli)

## Contributing

The Ninja MCP server welcomes improvements. Consider contributing:
* New tools for emerging Ninja features
* Better error messages
* Performance optimizations
* Integration examples

---

**Ninja MCP**: Give your AI assistants the power to manage your infrastructure.
