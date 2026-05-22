# Ninja CLI: Command-Line Control

The command-line interface for Ninja service management. It's your direct line to managing all your shurikens without leaving the terminal.

## Installation

The CLI is built in Rust and can be installed directly:

```bash
cargo install --path ./
```

Or as part of the main workspace:

```bash
cargo install --path ../CLI
```

Once installed, `shurikenctl` will be available in your PATH.

## Getting Started

### Interactive REPL Mode

Simply run `shurikenctl` with no arguments to enter the interactive REPL:

```bash
shurikenctl
ninja> list
ninja> start webserver
ninja> exit
```

### One-Off Commands

Execute commands directly:

```bash
shurikenctl list                          # See all installed shurikens
shurikenctl install ./service.shuriken    # Install a new shuriken
shurikenctl start webserver               # Start a shuriken
shurikenctl stop webserver                # Stop a running shuriken
shurikenctl configure webserver           # Generate configuration
```

## Command Reference

### list
List all installed shurikens and their current states.

```bash
shurikenctl list
shurikenctl list --state      # Show detailed state information
```

Output shows shuriken name and state: `Running`, `Idle`, or `Error`.

### install
Install a `.shuriken` package from a local or remote path.

```bash
shurikenctl install ./service.shuriken
shurikenctl install ~/Downloads/example-app-linux-x86_64.shuriken
```

The installation process:
1. Validates magic bytes and metadata
2. Verifies SHA-256 signature
3. Extracts to `~/.ninja/shurikens/{name}/`
4. Runs post-install script if specified

### remove
Uninstall a shuriken completely.

```bash
shurikenctl remove example-service
```

Warning: This deletes all shuriken files. Backup important data first.

### start
Start a shuriken service.

```bash
shurikenctl start webserver
```

Behavior depends on shuriken type:
* Native: Spawns process and creates lock file
* Script: Executes `start()` function in management script

### stop
Stop a running shuriken.

```bash
shurikenctl stop webserver
```

Behavior depends on shuriken type:
* Native: Terminates process using PID verification
* Script: Executes `stop()` function in management script

### restart
Restart a shuriken (stop then start).

```bash
shurikenctl restart database
```

### configure
Generate configuration from template and injected options.

```bash
shurikenctl configure webserver
```

Process steps:
1. Loads `.ninja/options.toml`
2. Injects `{{ platform }}` and `{{ root }}` variables
3. Renders `.ninja/config.tmpl`
4. Writes to configured path

### forge
Package a shuriken directory into a `.shuriken` binary.

```bash
shurikenctl forge              # Interactive mode
shurikenctl forge ./my-service # Specify source directory
```

Creates: `~/.ninja/blacksmith/{id}-{platform}.shuriken`

Interactive prompts:
* Name and ID
* Version
* Platform
* Synopsis and description
* Authors and license

### new
Create a new shuriken manifest interactively.

```bash
shurikenctl new
```

Creates `.ninja/manifest.toml` with guided prompts for all required fields.

### run
Execute a Ninja script (Lua or DSL).

```bash
shurikenctl run setup.ns
shurikenctl run scripts/deploy.lua
```

Scripts execute with full Ninja API access for dynamic automation.

### api
Start the HTTP API server for remote management.

```bash
shurikenctl api
shurikenctl api --port 3000
```

Options:
- `--port <PORT>` - Server port (default: 8080)

In another terminal, interact with the API:
```bash
curl http://localhost:8080/shurikens
curl -X POST http://localhost:8080/shurikens/myapp/start
```

See [API Reference](https://ninja-rs.vercel.app/docs/reference/api-reference) for full endpoint documentation.

### lockpick
Remove a stale lock file for a shuriken.

```bash
shurikenctl lockpick webserver
```

Warning: Only use if you've verified the process is not running. Misuse can cause race conditions.

Safe usage:
```bash
# 1. Verify process is stopped
ps aux | grep service-name

# 2. Stop if needed
shurikenctl stop service-name

# 3. Remove stale lock
shurikenctl lockpick service-name
```

## Common Workflows

### Complete Lifecycle

```bash
# Install package
shurikenctl install ./myapp.shuriken

# Configure
shurikenctl configure myapp

# Start
shurikenctl start myapp

# Check status
shurikenctl list

# Stop when done
shurikenctl stop myapp
```

### Package Development

```bash
# Create new shuriken
shurikenctl new

# Test locally
shurikenctl start myapp

# View logs if needed
shurikenctl list --state

# Build package
shurikenctl forge ./myapp

# Package created at:
# ~/.ninja/blacksmith/com.example.myapp-linux-x86_64.shuriken
```

### Running an API Server

```bash
# Terminal 1: Start server
shurikenctl api --port 8080

# Terminal 2: Use curl
curl http://localhost:8080/shurikens
curl -X POST http://localhost:8080/shurikens/myapp/start
curl -X GET http://localhost:8080/shurikens/myapp/status
```

## Global Options

```bash
shurikenctl [OPTIONS] [COMMAND]
```

- `--version` - Show version information
- `--help` - Display help message

Get help for any specific command:

```bash
shurikenctl --help
shurikenctl install --help
shurikenctl forge --help
```

## Environment Variables

Configure CLI behavior through environment:

```bash
# Ninja directory (default: ~/.ninja)
export NINJA_DIR=/custom/path

# Log level (default: info)
export NINJA_LOG_LEVEL=debug
```

## Exit Codes

* `0` - Success
* `1` - General error
* `2` - Invalid arguments
* `3` - Service not found
* `4` - Service already running
* `5` - Service not running

## Documentation

For more detailed information:
- [DSL Syntax](https://ninja-rs.vercel.app/docs/reference/dsl-syntax)
- [Configuration Format](https://ninja-rs.vercel.app/docs/reference/config-format)
- [Lua API Reference](https://ninja-rs.vercel.app/docs/reference/lua-api)
- [API Reference](https://ninja-rs.vercel.app/docs/reference/api-reference)

## Development

Build the CLI in debug mode:

```bash
cargo build
./target/debug/shurikenctl --help
```

Build optimized release:

```bash
cargo build --release
./target/release/shurikenctl --help
```

Run tests:

```bash
cargo test
```

## Architecture

The CLI is structured around a command dispatcher that:
1. Parses arguments and builds command structs
2. Initializes the core Ninja system
3. Delegates to the appropriate manager functions
4. Formats and displays results
5. Returns appropriate exit codes

This keeps the CLI thin and focused while delegating all heavy lifting to the core library.

This keeps the CLI thin and focused while delegating all heavy lifting to the core library.

---

**shurikenctl**: Where managing services becomes second nature.
