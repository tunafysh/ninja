# Ninja Core

A powerful, cross-platform package manager and runtime for managing tools and plugins (called "shurikens").

## Overview

Ninja Core is a Rust library that provides a comprehensive framework for installing, configuring, and managing executable packages with embedded scripting, templating, and process management capabilities.

## Features

- 🎯 **Shuriken Management**: Install, configure, and run packages ("shurikens") with ease
- 🔄 **State Management**: Track and manage shuriken states (running, idle, error)
- 📜 **Lua Scripting**: Embedded Lua scripting engine for dynamic configuration and automation
- 🌐 **HTTP Server**: Built-in HTTP server for remote management and control
- 📦 **Backup & Restore**: Backup and restore shuriken configurations and states
- 🖥️ **Cross-Platform**: Full support for Linux, macOS, and Windows
- 📝 **Template Engine**: Tera-powered templating for configuration files
- 🔐 **Elevated Permissions**: Seamless admin/sudo execution across platforms
- 🌍 **Remote Downloads**: Download shurikens from registries with opendal support
- ⚙️ **DSL Support**: Domain-specific language for easy shuriken control

## Architecture

### Core Modules

- **`manager`**: Central management system for shurikens and processes
- **`shuriken`**: Package representation and execution logic
- **`scripting`**: Lua scripting engine, DSL parser, and template engine
- **`common`**: Shared types, configuration, error handling, and registry
- **`backup`**: Backup and restore functionality
- **`utils`**: Helper functions for downloads, file operations, and more

## Key Concepts

### Shurikens

A "shuriken" is a managed package that can be:
- Installed from local or remote sources
- Configured with platform-specific settings
- Started, stopped, and monitored
- Scripted with Lua hooks
- Backed up and restored

### States

Each shuriken can be in one of three states:
- **Running**: Active and executing
- **Idle**: Installed but not running
- **Error**: Failed state with error message

## Usage

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
ninja-core = { path = "path/to/ninja-core" }
```

### Basic Example

```rust
use ninja::manager::ShurikenManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a manager
    let manager = ShurikenManager::new().await?;
    
    // Install a shuriken
    manager.install_shuriken(path).await?;
    
    // Start it
    manager.start("my-shuriken").await?;
    
    Ok(())
}
```

### DSL Commands

The built-in DSL supports various commands:

```
select <name>          # Select a shuriken
start                  # Start selected shuriken
stop                   # Stop selected shuriken
configure              # Configure selected shuriken
set <key> <value>      # Set configuration value
list                   # List all shurikens
list-state             # List shurikens with states
install <path>         # Install a shuriken
execute <path>         # Execute a script
```

## Platform Support

### Linux
- Uses `pkexec` for elevated permissions
- Full process management with signals

### macOS
- Uses `osascript` for GUI elevation prompts
- Native process control

### Windows
- UAC integration for elevated permissions
- Windows API process management

## Dependencies

Key dependencies include:
- **tokio**: Async runtime
- **mlua**: Lua scripting engine
- **tera**: Template engine
- **serde**: Serialization framework
- **anyhow**: Error handling
- **opendal**: Unified data access layer
- **reqwest**: HTTP client
- **tar/flate2**: Archive handling

## Configuration

Shurikens are configured using TOML files with support for:
- Platform-specific paths
- Environment variables
- Template variables
- Nested configurations
- Custom fields

## Building

```bash
# Build the library
cargo build --release

# Run tests
cargo test

# Build documentation
cargo doc --open
```

## License

See the license file in the workspace root.

## Contributing

Contributions are welcome! Please follow the project's coding standards and include tests for new features.
