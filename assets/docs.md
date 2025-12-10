# Ninja Documentation

## Overview

Ninja is a service management system that uses **Shurikens** as its core abstraction. A Shuriken is a packaged, configurable service or application that Ninja can install, configure, start, stop, and manage.

## Core Concepts

### Shurikens

A **Shuriken** is a self-contained package that represents a service or application. Each shuriken contains:

- **Metadata**: Name, version, platform requirements, type
- **Management configuration**: How to start/stop the service
- **Optional configuration templates**: For generating config files
- **Optional tools**: Scripts that can be executed
- **Optional logs configuration**: Where to store logs

Shurikens are distributed as `.shuriken` files and installed into the Ninja environment.

### Shuriken Manager

The `ShurikenManager` is the central component responsible for:

- Installing and removing shurikens
- Starting and stopping services
- Managing configuration
- Tracking service states
- Managing spawned processes

### Ninja Engine

The scripting engine that executes Lua scripts with access to system APIs (filesystem, shell, processes, etc.). Used for:

- Custom start/stop logic in script-managed shurikens
- Post-installation scripts
- Tool execution
- DSL command processing

---

## Directory Structure

```
~/.ninja/
├── shurikens/           # Installed shurikens
│   ├── example-app/
│   │   ├── .ninja/      # Ninja metadata
│   │   │   ├── manifest.toml    # Shuriken definition
│   │   │   ├── options.toml     # User configuration
│   │   │   ├── config.tmpl      # Configuration template
│   │   │   ├── shuriken.lck     # Runtime lock file
│   │   │   └── *.ns             # Lua scripts (stored with .ns extension)
│   │   └── [shuriken files]
│   └── another-app/
├── projects/            # Project storage (app-specific)
└── blacksmith/          # Built .shuriken packages
```

---

## Manifest Format

The `manifest.toml` file defines a shuriken. It must be located at `.ninja/manifest.toml` within the shuriken directory.

### Basic Manifest Structure

```toml
[shuriken]
name = "example-service"
id = "com.example.service"
version = "1.0.0"
type = "service"              # or "application", "tool", etc.
require-admin = false         # Whether elevated privileges are needed

[shuriken.management]
type = "native"               # or "script"

# For native management:
[shuriken.management.bin-path]
linux = "/usr/bin/example"
windows = "C:\\Program Files\\Example\\example.exe"
macos = "/usr/local/bin/example"

args = ["--daemon", "--config", "config.ini"]

[shuriken.management.cwd]
linux = "/var/lib/example"
windows = "C:\\ProgramData\\Example"
macos = "/usr/local/var/example"

# Configuration (optional)
[config]
config-path = "config.ini"

# Logging (optional)
[logs]
log-path = "logs/service.log"

# Tools (optional)
[[tools]]
name = "backup"
script = "scripts/backup.lua"
description = "Create a backup of the service data"
```

### Management Types

#### 1. Native Management

For services that are managed by spawning a native binary:

```toml
[shuriken.management]
type = "native"

[shuriken.management.bin-path]
linux = "./bin/server"
windows = ".\\bin\\server.exe"

args = ["--port", "8080", "--config", "server.conf"]

[shuriken.management.cwd]
linux = "."
```

**How it works:**
- Ninja spawns the binary as a child process
- Tracks the process ID (PID) and start time
- Creates a lock file (`.ninja/shuriken.lck`) with process info
- On stop, terminates the process using PID + start time verification

#### 2. Script Management

For services that require custom start/stop logic:

```toml
[shuriken.management]
type = "script"
script-path = "scripts/management.lua"
```

The management script must define `start()` and `stop()` functions:

```lua
-- scripts/management.lua

function start()
    log.info("Starting service...")
    
    -- Custom startup logic
    shell.exec("systemctl start myservice")
    
    -- Or spawn a process
    proc.spawn("./bin/server --daemon")
    
    log.info("Service started")
end

function stop()
    log.info("Stopping service...")
    
    -- Custom shutdown logic
    shell.exec("systemctl stop myservice")
    
    -- Or kill by name
    proc.kill_name("server")
    
    log.info("Service stopped")
end
```

**Note**: Ninja stores Lua scripts with the `.ns` extension (Ninja Script) in the `.ninja/` directory, but they're standard Lua scripts.

---

## The .shuriken File Format

A `.shuriken` file is a custom binary package format:

```
[MAGIC]              6 bytes  - "HSRZEG"
[metadata_length]    2 bytes  - u16 little-endian
[metadata]           N bytes  - CBOR-encoded metadata
[archive_length]     4 bytes  - u32 little-endian
[archive]            N bytes  - tar.gz compressed files
[signature]          32 bytes - SHA-256 hash of archive
```

### Metadata Structure

The CBOR-encoded metadata contains:

```rust
{
    "name": "example-service",
    "id": "com.example.service",
    "platform": "linux-x86_64",  // or "windows-x86_64", "macos-aarch64", "any"
    "version": "1.0.0",
    "synopsis": "A short description",
    "description": "Longer description...",
    "authors": ["Author Name"],
    "license": "MIT",
    "postinstall": "scripts/postinstall.lua"  // Optional
}
```

### Installation Process

1. **Validation**: Check magic bytes, parse metadata
2. **Platform check**: Verify platform compatibility
3. **Archive extraction**: Decompress tar.gz to `~/.ninja/shurikens/{name}/`
4. **Signature verification**: Validate archive integrity with SHA-256
5. **Post-install**: Run post-installation script if specified

---

## Configuration System

Shurikens can define configurable options that are rendered into configuration files using the Tera templating engine.

### Configuration Template Example

`.ninja/config.tmpl`:

```ini
[server]
host = {{ host }}
port = {{ port }}
debug = {{ debug }}

[database]
url = {{ db_url }}
pool_size = {{ db_pool_size }}

[paths]
data_dir = {{ root }}/data
log_dir = {{ root }}/logs

[system]
platform = {{ platform }}
```

### Options File

`.ninja/options.toml`:

```toml
host = "localhost"
port = 8080
debug = true
db_url = "postgresql://localhost/mydb"
db_pool_size = 10
```

### Injected Variables

Ninja automatically injects:

- `{{ platform }}`: Current OS (linux, windows, macos)
- `{{ root }}`: Absolute path to shuriken directory

### Generating Configuration

When `configure` is called, Ninja:

1. Loads options from `options.toml`
2. Injects default variables
3. Renders `.ninja/config.tmpl`
4. Writes output to the path specified in `config-path`

---

## State Management

### Shuriken States

- **Idle**: Not running
- **Running**: Currently active

### Lock File Format

When a shuriken is started, a lock file is created at `.ninja/shuriken.lck`:

**For Native Management:**
```json
{
  "name": "example-service",
  "type": "Native",
  "pid": 12345,
  "start_time": 1638360000
}
```

**For Script Management:**
```json
{
  "name": "example-service",
  "type": "Script"
}
```

### Process Verification

To prevent killing the wrong process, Ninja uses **PID + start time** verification:

1. Read PID from lock file
2. Get actual process start time from the system
3. Compare with recorded start time
4. Only kill if both match

This ensures robustness across process ID reuse.

---

## Tools

Shurikens can define executable tools:

```toml
[[tools]]
name = "migrate"
script = "scripts/migrate.lua"
description = "Run database migrations"

[[tools]]
name = "backup"
script = "scripts/backup.lua"
description = "Backup application data"
```

Tools are Lua scripts with access to the full Ninja API and can be executed via the DSL or API.

---

## Platform Paths

The `PlatformPath` type allows platform-specific paths:

```toml
[shuriken.management.bin-path]
linux = "/usr/bin/app"
windows = "C:\\Program Files\\App\\app.exe"
macos = "/Applications/App.app/Contents/MacOS/app"
```

At runtime, Ninja selects the appropriate path for the current platform.

---

## Admin/Elevated Privileges

When `require-admin = true`:

**Linux**: Uses `pkexec` to run the binary with elevated privileges
**macOS**: Uses `osascript` to trigger GUI password prompt
**Windows**: Uses PowerShell `Start-Process -Verb RunAs` to trigger UAC

---

## Example: Complete Shuriken

### Directory Structure

```
my-web-server/
├── .ninja/
│   ├── manifest.toml
│   ├── config.tmpl
│   └── scripts/
│       └── postinstall.lua
├── bin/
│   ├── server
│   └── server.exe
├── data/
└── README.md
```

### manifest.toml

```toml
[shuriken]
name = "my-web-server"
id = "com.mycompany.webserver"
version = "2.1.0"
type = "service"
require-admin = false

[shuriken.management]
type = "native"

[shuriken.management.bin-path]
linux = "./bin/server"
windows = ".\\bin\\server.exe"
macos = "./bin/server"

args = ["--config", "server.conf"]

[shuriken.management.cwd]
linux = "."
windows = "."
macos = "."

[config]
config-path = "server.conf"

[logs]
log-path = "logs/server.log"

[[tools]]
name = "status"
script = "scripts/status.lua"
description = "Check server status"
```

### config.tmpl

```ini
[server]
bind = {{ bind_address }}:{{ port }}
workers = {{ worker_count }}
log_level = {{ log_level }}

[paths]
static = {{ root }}/static
data = {{ root }}/data
logs = {{ root }}/logs

[security]
enable_tls = {{ enable_tls }}
cert_file = {{ cert_file }}
key_file = {{ key_file }}
```

### postinstall.lua

```lua
log.info("Running post-install setup...")

-- Create required directories
fs.create_dir("data")
fs.create_dir("logs")
fs.create_dir("static")

-- Set default permissions (Unix)
if env.os ~= "windows" then
    shell.exec("chmod +x bin/server")
end

log.info("Post-install complete!")
```

---

## Building a .shuriken Package

Use the `forge` command to create a distributable package:

```rust
let metadata = ArmoryMetadata {
    name: "my-web-server".to_string(),
    id: "com.mycompany.webserver".to_string(),
    platform: "linux-x86_64".to_string(),
    version: "2.1.0".to_string(),
    synopsis: Some("A high-performance web server".to_string()),
    postinstall: Some(PathBuf::from("scripts/postinstall.lua")),
    description: Some("Full description here...".to_string()),
    authors: Some(vec!["Your Name".to_string()]),
    license: Some("MIT".to_string()),
};

manager.forge(metadata, PathBuf::from("my-web-server")).await?;
```

This creates: `blacksmith/com.mycompany.webserver-linux-x86_64.shuriken`

---

## Best Practices

### 1. Always Use Relative Paths in Manifests

```toml
# Good
[shuriken.management.bin-path]
linux = "./bin/server"

# Avoid (unless necessary)
[shuriken.management.bin-path]
linux = "/absolute/path/server"
```

### 2. Provide Platform-Specific Configurations

```toml
[shuriken.management.bin-path]
linux = "./bin/server"
windows = ".\\bin\\server.exe"
macos = "./bin/server"
```

### 3. Use Script Management for Complex Services

If your service requires:
- Multiple processes
- Custom startup sequences
- Graceful shutdown logic
- Health checks

→ Use script management instead of native.

### 4. Include Post-Install Scripts

Use post-install scripts to:
- Create required directories
- Set file permissions
- Initialize databases
- Download dependencies

### 5. Version Your Shurikens

Always include a version in metadata and consider semantic versioning.

---

## Troubleshooting

### Shuriken Won't Start

1. Check lock file exists: `.ninja/shuriken.lck`
2. Verify binary permissions (Unix): `ls -l bin/server`
3. Check logs if configured
4. Test binary manually: `./bin/server --config config.ini`

### Configuration Not Generated

1. Verify `.ninja/config.tmpl` exists
2. Check options.toml syntax
3. Review Tera template syntax
4. Look for undefined variables in template

### Process Verification Fails

If "Failed to terminate shuriken" error occurs:
- Process may have crashed
- PID reused by another process
- Lock file out of sync

**Solution**: Manually remove `.ninja/shuriken.lck` and check actual processes